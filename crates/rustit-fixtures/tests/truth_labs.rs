use rustit_adapters::{ExternalIdentity, ExternalSystem};
use rustit_core::{
    ElementActivityLink, ElementActivityLinkId, ElementActivityRole, Project, ProjectId,
};
use rustit_geometry::{GeometryKernel, Mesh, Point3, Segment3};
use rustit_geometry_truck::TruckGeometryKernel;
use rustit_ifc::{IfcEntity, IfcSchemaVersion};
use rustit_model::{ElementId, Level, LevelId, Wall};
use rustit_schedule::{
    Activity, ActivityId, ActivityRelationship, RelationshipType, Schedule, calculate_cpm,
};
use serde::Deserialize;
use uuid::Uuid;

const STRAIGHT_WALL: &str = include_str!("../../../fixtures/wall-lab/straight-wall-a.json");
const INVALID_WALLS: &str = include_str!("../../../fixtures/wall-lab/invalid-walls.json");
const RELATIONSHIPS: &str = include_str!("../../../fixtures/schedule-lab/relationships.json");
const WALL_4D_LINK: &str = include_str!("../../../fixtures/4d-lab/wall-construction-link.json");
const IFC_MAPPING: &str = include_str!("../../../fixtures/ifc-lab/entity-mapping.json");
const P6_IDENTITY: &str = include_str!("../../../fixtures/sync-lab/p6-external-identity.json");

#[derive(Debug, Deserialize)]
struct StraightWallFixture {
    case: String,
    units: String,
    level_id: String,
    wall_id: String,
    baseline_start: [f64; 3],
    baseline_end: [f64; 3],
    thickness: f64,
    height: f64,
    expected_minimum: [f64; 3],
    expected_maximum: [f64; 3],
    expected_triangle_count: usize,
}

#[derive(Debug, Deserialize)]
struct InvalidWallsFixture {
    case: String,
    units: String,
    cases: Vec<InvalidWallCase>,
}

#[derive(Debug, Deserialize)]
struct InvalidWallCase {
    name: String,
    baseline_start: [f64; 3],
    baseline_end: [f64; 3],
    thickness: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
struct ScheduleFixture {
    case: String,
    time_basis: String,
    cases: Vec<ScheduleCase>,
}

#[derive(Debug, Deserialize)]
struct ScheduleCase {
    name: String,
    relationship: FixtureRelationship,
    predecessor_duration: u32,
    successor_duration: u32,
    lag_hours: i32,
    expected_successor_early_start: i64,
    expected_successor_early_finish: i64,
    expected_project_duration: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureRelationship {
    FinishStart,
    StartStart,
    FinishFinish,
    StartFinish,
}

impl From<FixtureRelationship> for RelationshipType {
    fn from(value: FixtureRelationship) -> Self {
        match value {
            FixtureRelationship::FinishStart => Self::FinishStart,
            FixtureRelationship::StartStart => Self::StartStart,
            FixtureRelationship::FinishFinish => Self::FinishFinish,
            FixtureRelationship::StartFinish => Self::StartFinish,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FourDFixture {
    case: String,
    project_id: String,
    level_id: String,
    wall_id: String,
    activity_id: String,
    link_id: String,
    role: String,
    expected_ifc_entity: String,
}

#[derive(Debug, Deserialize)]
struct IfcMappingFixture {
    case: String,
    schema: String,
    serialization_status: String,
    mappings: Vec<IfcMapping>,
}

#[derive(Debug, Deserialize)]
struct IfcMapping {
    rustit_type: String,
    ifc_entity: String,
}

#[derive(Debug, Deserialize)]
struct SyncFixture {
    case: String,
    canonical_object_id: String,
    system: String,
    external_id: String,
    expected_canonical_object_id: String,
    expected_external_id: String,
}

#[test]
fn wall_lab_matches_authored_dimensions_and_complete_mesh() {
    let fixture: StraightWallFixture = serde_json::from_str(STRAIGHT_WALL).expect("wall fixture");
    assert_eq!(fixture.case, "straight-wall-a");
    assert_eq!(fixture.units, "metres");

    let mut wall = Wall::new(
        LevelId::from_uuid(parse_uuid(&fixture.level_id)),
        Segment3::new(point(fixture.baseline_start), point(fixture.baseline_end)),
        fixture.thickness,
        fixture.height,
    )
    .expect("fixture wall geometry");
    wall.id = ElementId::from_uuid(parse_uuid(&fixture.wall_id));

    let mesh = TruckGeometryKernel::default()
        .wall_mesh(wall.geometry())
        .expect("fixture wall mesh");
    let (minimum, maximum) = mesh_bounds(&mesh);

    assert_point(minimum, fixture.expected_minimum);
    assert_point(maximum, fixture.expected_maximum);
    assert_eq!(mesh.triangles.len(), fixture.expected_triangle_count);
    assert_eq!(wall.id.as_uuid(), parse_uuid(&fixture.wall_id));
}

#[test]
fn wall_lab_rejects_each_named_invalid_case() {
    let fixture: InvalidWallsFixture =
        serde_json::from_str(INVALID_WALLS).expect("invalid-wall fixture");
    assert_eq!(fixture.case, "invalid-wall-dimensions");
    assert_eq!(fixture.units, "metres");

    for case in fixture.cases {
        let result = Wall::new(
            LevelId::new(),
            Segment3::new(point(case.baseline_start), point(case.baseline_end)),
            case.thickness,
            case.height,
        );
        assert!(result.is_err(), "{} should be rejected", case.name);
    }
}

#[test]
fn schedule_lab_matches_independently_calculated_relationship_timings() {
    let fixture: ScheduleFixture = serde_json::from_str(RELATIONSHIPS).expect("schedule fixture");
    assert_eq!(fixture.case, "relationship-types-and-offsets");
    assert_eq!(fixture.time_basis, "working_hours_without_calendars");

    for case in fixture.cases {
        let predecessor = Activity::new("Predecessor", case.predecessor_duration);
        let successor = Activity::new("Successor", case.successor_duration);
        let successor_id = successor.id;
        let schedule = Schedule {
            activities: vec![predecessor.clone(), successor],
            relationships: vec![ActivityRelationship::new(
                predecessor.id,
                successor_id,
                case.relationship.into(),
                case.lag_hours,
            )],
        };

        let result = calculate_cpm(&schedule).expect("fixture schedule");
        let timing = result
            .timings
            .iter()
            .find(|timing| timing.activity_id == successor_id)
            .expect("successor timing");

        assert_eq!(
            timing.early_start, case.expected_successor_early_start,
            "{} early start",
            case.name
        );
        assert_eq!(
            timing.early_finish, case.expected_successor_early_finish,
            "{} early finish",
            case.name
        );
        assert_eq!(
            result.project_duration_hours, case.expected_project_duration,
            "{} project duration",
            case.name
        );
    }
}

#[test]
fn four_d_lab_preserves_all_stable_ids_in_a_vendor_neutral_link() {
    let fixture: FourDFixture = serde_json::from_str(WALL_4D_LINK).expect("4D fixture");
    assert_eq!(fixture.case, "wall-construction-link");
    assert_eq!(fixture.role, "construct");

    let mut project = Project::new("4D Lab");
    project.id = ProjectId::from_uuid(parse_uuid(&fixture.project_id));
    let mut level = Level::new("Level 1", 0.0);
    level.id = LevelId::from_uuid(parse_uuid(&fixture.level_id));
    let mut wall = Wall::new(
        level.id,
        Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(6.0, 0.0, 0.0)),
        0.2,
        3.0,
    )
    .expect("4D fixture wall");
    wall.id = ElementId::from_uuid(parse_uuid(&fixture.wall_id));
    let mut activity = Activity::new("Construct wall", 8);
    activity.id = ActivityId::from_uuid(parse_uuid(&fixture.activity_id));
    let mut link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);
    link.id = ElementActivityLinkId::from_uuid(parse_uuid(&fixture.link_id));

    project.model.add_level(level).expect("fixture level");
    project.model.add_wall(wall).expect("fixture wall");
    project
        .schedule
        .add_activity(activity)
        .expect("fixture activity");
    project
        .link_element_activity(link.clone())
        .expect("fixture 4D link");

    assert_eq!(project.id.as_uuid(), parse_uuid(&fixture.project_id));
    assert_eq!(link.element_id.as_uuid(), parse_uuid(&fixture.wall_id));
    assert_eq!(link.activity_id.as_uuid(), parse_uuid(&fixture.activity_id));
    assert_eq!(link.id.as_uuid(), parse_uuid(&fixture.link_id));
    assert_eq!(link.ifc_entity().schema_name(), fixture.expected_ifc_entity);
}

#[test]
fn ifc_lab_matches_the_declared_semantic_mapping_without_claiming_serialization() {
    let fixture: IfcMappingFixture = serde_json::from_str(IFC_MAPPING).expect("IFC fixture");
    assert_eq!(fixture.case, "implemented-semantic-entity-map");
    assert_eq!(
        IfcSchemaVersion::Ifc4x3Add2.schema_identifier(),
        fixture.schema
    );
    assert_eq!(fixture.serialization_status, "not_implemented");

    for mapping in fixture.mappings {
        let entity = match mapping.rustit_type.as_str() {
            "Project" => IfcEntity::Project,
            "Level" => IfcEntity::BuildingStorey,
            "Wall" => IfcEntity::Wall,
            "Activity" => IfcEntity::Task,
            "ActivityRelationship" => IfcEntity::RelSequence,
            "ElementActivityLink" => IfcEntity::RelAssignsToProcess,
            unknown => panic!("fixture names unknown Rustit type {unknown}"),
        };
        assert_eq!(entity.schema_name(), mapping.ifc_entity);
    }
}

#[test]
fn sync_lab_keeps_the_canonical_uuid_separate_from_the_p6_identifier() {
    let fixture: SyncFixture = serde_json::from_str(P6_IDENTITY).expect("sync fixture");
    assert_eq!(fixture.case, "p6-external-identity");
    assert_eq!(fixture.system, "primavera_p6");
    let identity = ExternalIdentity {
        object_id: parse_uuid(&fixture.canonical_object_id),
        system: ExternalSystem::PrimaveraP6,
        external_id: fixture.external_id,
    };

    let encoded = serde_json::to_string(&identity).expect("serialize external identity");
    let restored: ExternalIdentity =
        serde_json::from_str(&encoded).expect("restore external identity");

    assert_eq!(
        restored.object_id,
        parse_uuid(&fixture.expected_canonical_object_id)
    );
    assert_eq!(restored.external_id, fixture.expected_external_id);
    assert_eq!(restored.system, ExternalSystem::PrimaveraP6);
}

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("fixture UUID")
}

fn point(value: [f64; 3]) -> Point3 {
    Point3::new(value[0], value[1], value[2])
}

fn mesh_bounds(mesh: &Mesh) -> (Point3, Point3) {
    mesh.vertices.iter().fold(
        (
            Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(minimum, maximum), point| {
            (
                Point3::new(
                    minimum.x.min(point.x),
                    minimum.y.min(point.y),
                    minimum.z.min(point.z),
                ),
                Point3::new(
                    maximum.x.max(point.x),
                    maximum.y.max(point.y),
                    maximum.z.max(point.z),
                ),
            )
        },
    )
}

fn assert_point(actual: Point3, expected: [f64; 3]) {
    const EPSILON: f64 = 1.0e-9;
    assert!((actual.x - expected[0]).abs() < EPSILON);
    assert!((actual.y - expected[1]).abs() < EPSILON);
    assert!((actual.z - expected[2]).abs() < EPSILON);
}
