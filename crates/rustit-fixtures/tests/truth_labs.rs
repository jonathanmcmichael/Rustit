use rustit_adapters::{ExternalIdentity, ExternalSystem};
use rustit_core::{
    ElementActivityLink, ElementActivityLinkId, ElementActivityRole, Project, ProjectId,
};
use rustit_geometry::{GeometryKernel, Mesh, Point3, Segment3};
use rustit_geometry_truck::TruckGeometryKernel;
use rustit_ifc::{
    ClassificationError, ClassificationReference, ClassificationSystem, IfcEntity, IfcSchemaVersion,
};
use rustit_model::{ElementId, Level, LevelId, Wall};
use rustit_schedule::{
    Activity, ActivityId, ActivityRelationship, RelationshipType, Schedule, calculate_cpm,
};
use serde::Deserialize;
use uuid::Uuid;

const STRAIGHT_WALL: &str = include_str!("../../../fixtures/wall-lab/straight-wall-a.json");
const INVALID_WALLS: &str = include_str!("../../../fixtures/wall-lab/invalid-walls.json");
const CLASSIFICATION_PROVENANCE: &str =
    include_str!("../../../fixtures/wall-lab/classification-provenance.json");
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

#[derive(Debug, Deserialize)]
struct ClassificationProvenanceFixture {
    case: String,
    units: String,
    level_id: String,
    wall_id: String,
    baseline_start: [f64; 3],
    baseline_end: [f64; 3],
    thickness: f64,
    height: f64,
    references: Vec<FixtureClassification>,
    duplicate_reference_index: usize,
    expected_classification_count: usize,
    blank_identification: String,
}

#[derive(Debug, Deserialize, Clone)]
struct FixtureClassification {
    system: String,
    edition: String,
    identification: String,
    name: String,
}

impl FixtureClassification {
    fn into_reference(self) -> ClassificationReference {
        match self.system.as_str() {
            "MasterFormat" => {
                ClassificationReference::master_format(self.edition, self.identification, self.name)
                    .expect("valid MasterFormat fixture reference")
            }
            "UniFormat" => {
                ClassificationReference::uni_format(self.edition, self.identification, self.name)
                    .expect("valid UniFormat fixture reference")
            }
            other => panic!("fixture names unknown classification system {other}"),
        }
    }
}

#[test]
fn wall_lab_matches_authored_dimensions_and_complete_mesh() {
    let fixture: StraightWallFixture = serde_json::from_str(STRAIGHT_WALL).expect("wall fixture");
    assert_eq!(fixture.case, "princess-donut-straight-wall");
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
    assert_eq!(fixture.case, "dungeon-invalid-wall-dimensions");
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
fn wall_lab_retains_masterformat_and_uniformat_provenance_without_catalogs() {
    let fixture: ClassificationProvenanceFixture =
        serde_json::from_str(CLASSIFICATION_PROVENANCE).expect("classification provenance fixture");
    assert_eq!(fixture.case, "carl-classification-provenance");
    assert_eq!(fixture.units, "metres");
    assert_eq!(fixture.references.len(), 2);

    let mut wall = Wall::new(
        LevelId::from_uuid(parse_uuid(&fixture.level_id)),
        Segment3::new(point(fixture.baseline_start), point(fixture.baseline_end)),
        fixture.thickness,
        fixture.height,
    )
    .expect("fixture wall geometry");
    wall.id = ElementId::from_uuid(parse_uuid(&fixture.wall_id));

    for reference in fixture.references.clone() {
        wall.add_classification(reference.into_reference());
    }

    let duplicate = fixture.references[fixture.duplicate_reference_index]
        .clone()
        .into_reference();
    wall.add_classification(duplicate);

    assert_eq!(
        wall.classifications.len(),
        fixture.expected_classification_count
    );
    assert_eq!(
        wall.classifications[0].system,
        ClassificationSystem::MasterFormat
    );
    assert_eq!(
        wall.classifications[0].edition.as_deref(),
        Some("project edition")
    );
    assert_eq!(wall.classifications[0].identification, "07 00 00");
    assert_eq!(
        wall.classifications[0].name.as_deref(),
        Some("Thermal and Moisture Protection")
    );
    assert_eq!(
        wall.classifications[1].system,
        ClassificationSystem::UniFormat
    );
    assert_eq!(
        wall.classifications[1].edition.as_deref(),
        Some("project edition")
    );
    assert_eq!(wall.classifications[1].identification, "B2010");
    assert_eq!(
        wall.classifications[1].name.as_deref(),
        Some("Exterior Walls")
    );

    assert_eq!(
        ClassificationReference::new(
            ClassificationSystem::MasterFormat,
            Some("project edition".into()),
            fixture.blank_identification,
            Some("ignored".into()),
        ),
        Err(ClassificationError::EmptyIdentification)
    );

    let encoded = serde_json::to_string(&wall).expect("serialize classified wall");
    let restored: Wall = serde_json::from_str(&encoded).expect("restore classified wall");
    assert_eq!(restored.id, wall.id);
    assert_eq!(restored.classifications, wall.classifications);
    assert_eq!(restored.id.as_uuid(), parse_uuid(&fixture.wall_id));
}

#[test]
fn schedule_lab_matches_independently_calculated_relationship_timings() {
    let fixture: ScheduleFixture = serde_json::from_str(RELATIONSHIPS).expect("schedule fixture");
    assert_eq!(fixture.case, "mordecai-cpm-relationship-tutorial");
    assert_eq!(fixture.time_basis, "working_hours_without_calendars");

    for case in fixture.cases {
        let predecessor = Activity::new("Carl predecessor", case.predecessor_duration);
        let successor = Activity::new("Princess Donut successor", case.successor_duration);
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
fn schedule_lab_reports_exactly_the_members_of_a_cyclic_network() {
    const CYCLIC_NETWORK: &str = include_str!("../../../fixtures/schedule-lab/cyclic-network.json");

    #[derive(Debug, Deserialize)]
    struct CyclicFixture {
        case: String,
        time_basis: String,
        activities: Vec<CyclicActivity>,
        relationships: Vec<CyclicRelationship>,
        expected_cycles: Vec<Vec<String>>,
        expected_excluded: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct CyclicActivity {
        id: String,
        name: String,
        duration_hours: u32,
    }

    #[derive(Debug, Deserialize)]
    struct CyclicRelationship {
        predecessor_id: String,
        successor_id: String,
    }

    let fixture: CyclicFixture =
        serde_json::from_str(CYCLIC_NETWORK).expect("cyclic network fixture");
    assert_eq!(fixture.case, "mordecai-cyclic-network-diagnostic");
    assert_eq!(fixture.time_basis, "working_hours_without_calendars");

    let activity_id = |value: &str| ActivityId::from_uuid(parse_uuid(value));
    let mut schedule = Schedule::default();
    for fixture_activity in &fixture.activities {
        let mut activity = Activity::new(&fixture_activity.name, fixture_activity.duration_hours);
        activity.id = activity_id(&fixture_activity.id);
        schedule.add_activity(activity).expect("fixture activity");
    }
    for relationship in &fixture.relationships {
        schedule
            .add_relationship(ActivityRelationship::new(
                activity_id(&relationship.predecessor_id),
                activity_id(&relationship.successor_id),
                RelationshipType::FinishStart,
                0,
            ))
            .expect("fixture relationship");
    }

    let expected: Vec<Vec<ActivityId>> = fixture
        .expected_cycles
        .iter()
        .map(|cycle| cycle.iter().map(|id| activity_id(id)).collect())
        .collect();
    let cycles = match calculate_cpm(&schedule) {
        Err(rustit_schedule::ScheduleError::CyclicNetwork { cycles }) => cycles,
        other => panic!("expected a cyclic network diagnostic, got {other:?}"),
    };
    assert_eq!(cycles, expected);
    for excluded in &fixture.expected_excluded {
        let excluded = activity_id(excluded);
        assert!(
            cycles.iter().flatten().all(|id| *id != excluded),
            "{excluded} must not be reported as a cycle member"
        );
    }

    // Repeated calculation returns an identical diagnostic.
    assert_eq!(
        calculate_cpm(&schedule),
        Err(rustit_schedule::ScheduleError::CyclicNetwork { cycles })
    );
}

#[test]
fn four_d_lab_preserves_all_stable_ids_in_a_vendor_neutral_link() {
    let fixture: FourDFixture = serde_json::from_str(WALL_4D_LINK).expect("4D fixture");
    assert_eq!(fixture.case, "royal-court-wall-construction-link");
    assert_eq!(fixture.role, "construct");

    let mut project = Project::new("The Royal Court 4D Lab");
    project.id = ProjectId::from_uuid(parse_uuid(&fixture.project_id));
    let mut level = Level::new("Dungeon Floor 1", 0.0);
    level.id = LevelId::from_uuid(parse_uuid(&fixture.level_id));
    let mut wall = Wall::new(
        level.id,
        Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(6.0, 0.0, 0.0)),
        0.2,
        3.0,
    )
    .expect("4D fixture wall");
    wall.id = ElementId::from_uuid(parse_uuid(&fixture.wall_id));
    let mut activity = Activity::new("Carl constructs wall", 8);
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
    assert_eq!(fixture.case, "crawler-semantic-entity-map");
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
    assert_eq!(fixture.case, "borant-p6-external-identity");
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

/// RFC 0004 acceptance fixtures 1–4: separated edits, overlapping edits,
/// atomic rejection, and no implicit cascades.
#[test]
fn transaction_lab_matches_rfc_0004_concurrency_rules() {
    use rustit_core::{
        Actor, ActorKind, CommitError, Domain, ObjectRef, ObjectVersion, Operation, OperationError,
        ProjectSnapshot, Proposal,
    };

    const CONCURRENT_EDITS: &str =
        include_str!("../../../fixtures/transaction-lab/concurrent-edits.json");

    #[derive(Debug, Deserialize)]
    struct Fixture {
        case: String,
        rfc: String,
        units: String,
        project_id: String,
        level_id: String,
        wall_id: String,
        activity_id: String,
        link_id: String,
        wall_baseline_start: [f64; 3],
        wall_baseline_end: [f64; 3],
        wall_thickness: f64,
        wall_height: f64,
        activity_duration_hours: u32,
        separated_edits: Separated,
        overlapping_edits: Overlapping,
        atomic_rejection: Atomic,
        no_cascades: NoCascades,
    }

    #[derive(Debug, Deserialize)]
    struct Separated {
        modeler_wall_height: f64,
        scheduler_activity_duration_hours: u32,
        expected_head: u64,
        expected_wall_version: u64,
        expected_activity_version: u64,
        expected_link_version: u64,
        expected_domains: Vec<Vec<String>>,
    }

    #[derive(Debug, Deserialize)]
    struct Overlapping {
        first_wall_height: f64,
        second_wall_height: f64,
        expected_conflicts: Vec<FixtureRef>,
        expected_wall_height: f64,
        expected_wall_version: u64,
        expected_head: u64,
    }

    #[derive(Debug, Deserialize)]
    struct Atomic {
        activity_duration_hours: u32,
        valid_wall_height: f64,
        invalid_wall_height: f64,
        expected_rejected_operation_index: usize,
    }

    #[derive(Debug, Deserialize)]
    struct NoCascades {
        expected_dependents: Vec<FixtureRef>,
        expected_change_order: Vec<FixtureRef>,
        expected_head: u64,
        expected_wall_tombstone_version: u64,
        expected_link_tombstone_version: u64,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureRef {
        kind: String,
        id: String,
    }

    fn object_ref(value: &FixtureRef) -> ObjectRef {
        let uuid = parse_uuid(&value.id);
        match value.kind.as_str() {
            "wall" => ObjectRef::Wall(ElementId::from_uuid(uuid)),
            "activity" => ObjectRef::Activity(ActivityId::from_uuid(uuid)),
            "link" => ObjectRef::Link(ElementActivityLinkId::from_uuid(uuid)),
            other => panic!("unsupported fixture object kind {other}"),
        }
    }

    fn domain_name(domain: Domain) -> &'static str {
        match domain {
            Domain::Project => "project",
            Domain::Model => "model",
            Domain::Schedule => "schedule",
            Domain::FourD => "four_d",
        }
    }

    let fixture: Fixture = serde_json::from_str(CONCURRENT_EDITS).expect("transaction fixture");
    assert_eq!(fixture.case, "safe-room-concurrent-edits");
    assert_eq!(fixture.rfc, "0004");
    assert_eq!(fixture.units, "metres_and_working_hours");

    let wall_id = ElementId::from_uuid(parse_uuid(&fixture.wall_id));
    let activity_id = ActivityId::from_uuid(parse_uuid(&fixture.activity_id));
    let link_id = ElementActivityLinkId::from_uuid(parse_uuid(&fixture.link_id));
    let wall_ref = ObjectRef::Wall(wall_id);
    let activity_ref = ObjectRef::Activity(activity_id);
    let link_ref = ObjectRef::Link(link_id);

    let genesis = || {
        let mut project = Project::new("Safe Room transaction lab");
        project.id = ProjectId::from_uuid(parse_uuid(&fixture.project_id));
        let mut level = Level::new("Dungeon Floor 1", 0.0);
        level.id = LevelId::from_uuid(parse_uuid(&fixture.level_id));
        let mut wall = Wall::new(
            level.id,
            Segment3::new(
                point(fixture.wall_baseline_start),
                point(fixture.wall_baseline_end),
            ),
            fixture.wall_thickness,
            fixture.wall_height,
        )
        .expect("fixture wall");
        wall.id = wall_id;
        let mut activity = Activity::new("Carl frames wall", fixture.activity_duration_hours);
        activity.id = activity_id;
        let mut link =
            ElementActivityLink::new(wall_id, activity_id, ElementActivityRole::Construct);
        link.id = link_id;

        project.model.add_level(level).expect("fixture level");
        project.model.add_wall(wall).expect("fixture wall");
        project
            .schedule
            .add_activity(activity)
            .expect("fixture activity");
        project.link_element_activity(link).expect("fixture link");
        ProjectSnapshot::genesis(project)
    };
    let modeler = Actor::new(ActorKind::Human, "princess-donut-modeler");
    let scheduler = Actor::new(ActorKind::Human, "mordecai-scheduler");
    let wall_height = |snapshot: &ProjectSnapshot| snapshot.project().model.walls[0].height;

    // 1. Separated edits.
    let separated = &fixture.separated_edits;
    let mut snapshot = genesis();
    let model_edit = snapshot
        .preview(Proposal::new(
            modeler.clone(),
            "Raise wall to clear the ceiling plenum",
            vec![Operation::SetWallHeight {
                id: wall_id,
                height: separated.modeler_wall_height,
            }],
        ))
        .expect("modeler preview");
    let schedule_edit = snapshot
        .preview(Proposal::new(
            scheduler.clone(),
            "Framing crew reduced",
            vec![Operation::SetActivityDuration {
                id: activity_id,
                duration_hours: separated.scheduler_activity_duration_hours,
            }],
        ))
        .expect("scheduler preview");
    let first = snapshot.commit(model_edit).expect("modeler commit");
    let second = snapshot
        .commit(schedule_edit)
        .expect("scheduler commit does not conflict");

    assert_eq!(snapshot.head().0, separated.expected_head);
    assert_eq!(
        snapshot.version(wall_ref),
        Some(ObjectVersion(separated.expected_wall_version))
    );
    assert_eq!(
        snapshot.version(activity_ref),
        Some(ObjectVersion(separated.expected_activity_version))
    );
    assert_eq!(
        snapshot.version(link_ref),
        Some(ObjectVersion(separated.expected_link_version))
    );
    let domains: Vec<Vec<&str>> = [&first, &second]
        .iter()
        .map(|entry| entry.domains.iter().copied().map(domain_name).collect())
        .collect();
    assert_eq!(domains, separated.expected_domains);
    assert_eq!(wall_height(&snapshot), separated.modeler_wall_height);
    assert_eq!(
        snapshot.project().schedule.activities[0].duration_hours,
        separated.scheduler_activity_duration_hours
    );

    // 2. Overlapping edits.
    let overlapping = &fixture.overlapping_edits;
    let mut snapshot = genesis();
    let set_height = |height: f64| {
        Proposal::new(
            modeler.clone(),
            "Adjust wall height",
            vec![Operation::SetWallHeight {
                id: wall_id,
                height,
            }],
        )
    };
    let winner = snapshot
        .preview(set_height(overlapping.first_wall_height))
        .expect("first preview");
    let loser = snapshot
        .preview(set_height(overlapping.second_wall_height))
        .expect("second preview");
    snapshot.commit(winner).expect("first commit");
    let before_conflict = snapshot.clone();

    assert_eq!(
        snapshot.commit(loser),
        Err(CommitError::Conflict {
            objects: overlapping
                .expected_conflicts
                .iter()
                .map(object_ref)
                .collect()
        })
    );
    assert_eq!(snapshot, before_conflict, "a conflict writes nothing");
    assert_eq!(wall_height(&snapshot), overlapping.expected_wall_height);
    assert_eq!(
        snapshot.version(wall_ref),
        Some(ObjectVersion(overlapping.expected_wall_version))
    );
    assert_eq!(snapshot.head().0, overlapping.expected_head);

    // 3. Atomic rejection.
    let atomic = &fixture.atomic_rejection;
    let snapshot = genesis();
    let rejection = snapshot
        .preview(Proposal::new(
            modeler.clone(),
            "Mixed edit with one invalid height",
            vec![
                Operation::SetActivityDuration {
                    id: activity_id,
                    duration_hours: atomic.activity_duration_hours,
                },
                Operation::SetWallHeight {
                    id: wall_id,
                    height: atomic.valid_wall_height,
                },
                Operation::SetWallHeight {
                    id: wall_id,
                    height: atomic.invalid_wall_height,
                },
            ],
        ))
        .expect_err("negative height rejects the proposal");

    assert_eq!(
        rejection.operation_index,
        atomic.expected_rejected_operation_index
    );
    assert!(matches!(rejection.error, OperationError::Model(_)));
    assert_eq!(snapshot, genesis(), "a rejected preview changes nothing");

    // 4. No implicit cascades.
    let no_cascades = &fixture.no_cascades;
    let mut snapshot = genesis();
    let rejection = snapshot
        .preview(Proposal::new(
            modeler.clone(),
            "Remove wall",
            vec![Operation::DeleteWall { id: wall_id }],
        ))
        .expect_err("a linked wall cannot be deleted implicitly");

    assert_eq!(rejection.operation_index, 0);
    assert_eq!(
        rejection.error,
        OperationError::HasDependents {
            object: wall_ref,
            dependents: no_cascades
                .expected_dependents
                .iter()
                .map(object_ref)
                .collect(),
        }
    );

    let explicit = snapshot
        .preview(Proposal::new(
            modeler,
            "Unlink, then remove wall",
            vec![
                Operation::UnlinkElementActivity { id: link_id },
                Operation::DeleteWall { id: wall_id },
            ],
        ))
        .expect("explicit unlink then delete");
    let changed: Vec<ObjectRef> = explicit
        .changes()
        .iter()
        .map(|change| change.object)
        .collect();
    assert_eq!(
        changed,
        no_cascades
            .expected_change_order
            .iter()
            .map(object_ref)
            .collect::<Vec<_>>()
    );
    assert!(
        explicit
            .changes()
            .iter()
            .all(|change| change.before.is_some() && change.after.is_none())
    );
    snapshot.commit(explicit).expect("explicit delete commits");

    assert_eq!(snapshot.head().0, no_cascades.expected_head);
    assert_eq!(snapshot.version(wall_ref), None);
    assert_eq!(
        snapshot.tombstone_version(wall_ref),
        Some(ObjectVersion(no_cascades.expected_wall_tombstone_version))
    );
    assert_eq!(
        snapshot.tombstone_version(link_ref),
        Some(ObjectVersion(no_cascades.expected_link_tombstone_version))
    );
    assert_eq!(snapshot.version(activity_ref), Some(ObjectVersion(1)));
    assert!(snapshot.project().element_activity_links.is_empty());
    assert!(snapshot.project().model.walls.is_empty());
}
