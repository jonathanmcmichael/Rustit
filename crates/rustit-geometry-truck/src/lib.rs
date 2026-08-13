//! Truck-backed implementation of Rustit's geometry contracts.

use rustit_geometry::{
    GeometryKernel, Mesh, Point3 as RustitPoint3, RectangularOpeningGeometry, WallGeometry,
};
use thiserror::Error;
use truck_meshalgo::prelude::*;
use truck_modeling::{Point3, Solid, Vector3, Wire, builder};
use truck_stepio::out;

/// First production candidate for Rustit's pure-Rust geometry kernel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TruckGeometryKernel {
    tessellation_tolerance: f64,
}

impl TruckGeometryKernel {
    pub fn new(tessellation_tolerance: f64) -> Result<Self, TruckGeometryError> {
        if !tessellation_tolerance.is_finite() || tessellation_tolerance <= TOLERANCE {
            return Err(TruckGeometryError::InvalidTessellationTolerance);
        }
        Ok(Self {
            tessellation_tolerance,
        })
    }

    #[must_use]
    pub const fn tessellation_tolerance(self) -> f64 {
        self.tessellation_tolerance
    }

    pub fn wall_solid(&self, wall: WallGeometry) -> Result<Solid, TruckGeometryError> {
        let (wall, origin, along, across) = wall_frame(wall)?;

        let vertex = builder::vertex(origin);
        let baseline_edge = builder::tsweep(&vertex, along);
        let footprint = builder::tsweep(&baseline_edge, across);
        Ok(builder::tsweep(
            &footprint,
            Vector3::new(0.0, 0.0, wall.height),
        ))
    }

    /// Builds a wall with one rectangular opening without requiring a Boolean
    /// subtraction. The opening becomes an inner boundary on the wall face and
    /// is swept through the full thickness.
    pub fn wall_solid_with_opening(
        &self,
        wall: WallGeometry,
        opening: RectangularOpeningGeometry,
    ) -> Result<Solid, TruckGeometryError> {
        let (wall, origin, along, across) = wall_frame(wall)?;
        let opening = opening.validate_for(wall)?;
        let length = wall.baseline.length();
        let along_unit = along / length;
        let up = Vector3::new(0.0, 0.0, 1.0);

        let vertex = builder::vertex(origin);
        let baseline_edge = builder::tsweep(&vertex, along);
        let mut elevation = builder::tsweep(&baseline_edge, Vector3::new(0.0, 0.0, wall.height));

        let lower_left = origin + along_unit * opening.offset + up * opening.sill_height;
        let lower_right = lower_left + along_unit * opening.width;
        let upper_right = lower_right + up * opening.height;
        let upper_left = lower_left + up * opening.height;
        let mut boundary = rectangular_wire([lower_left, lower_right, upper_right, upper_left]);
        boundary.invert();
        elevation.add_boundary(boundary);

        Ok(builder::tsweep(&elevation, across))
    }

    /// Joins two solids using Truck's experimental shape operations.
    pub fn union(&self, first: &Solid, second: &Solid) -> Result<Solid, TruckGeometryError> {
        truck_shapeops::or(first, second, self.tessellation_tolerance)
            .ok_or(TruckGeometryError::BooleanFailed)
    }

    /// Subtracts `tool` from `whole` using Truck's inverted-solid intersection
    /// convention.
    pub fn subtract(&self, whole: &Solid, tool: &Solid) -> Result<Solid, TruckGeometryError> {
        let mut inverted_tool = tool.clone();
        inverted_tool.not();
        truck_shapeops::and(whole, &inverted_tool, self.tessellation_tolerance)
            .ok_or(TruckGeometryError::BooleanFailed)
    }

    /// Exports a modeled solid as a STEP exchange document.
    ///
    /// Truck currently cannot export solids produced by shape operations, so
    /// callers should retain the pre-Boolean authored solid for this path.
    #[must_use]
    pub fn solid_to_step(&self, solid: &Solid) -> String {
        let compressed = solid.compress();
        out::CompleteStepDisplay::new(
            out::StepModel::from(&compressed),
            out::StepHeaderDescriptor {
                organization_system: "Rustit Truck geometry evaluation".to_owned(),
                ..Default::default()
            },
        )
        .to_string()
    }

    fn tessellate(&self, solid: &Solid) -> Mesh {
        let polygon = solid
            .triangulation(self.tessellation_tolerance)
            .to_polygon();
        let vertices = polygon
            .positions()
            .iter()
            .map(|position| RustitPoint3::new(position.x, position.y, position.z))
            .collect();
        let triangles = polygon
            .tri_faces()
            .iter()
            .map(|triangle| {
                [
                    triangle[0].pos as u32,
                    triangle[1].pos as u32,
                    triangle[2].pos as u32,
                ]
            })
            .collect();

        Mesh {
            vertices,
            triangles,
        }
    }
}

fn wall_frame(
    wall: WallGeometry,
) -> Result<(WallGeometry, Point3, Vector3, Vector3), TruckGeometryError> {
    let wall = wall.validate()?;
    let dx = wall.baseline.end.x - wall.baseline.start.x;
    let dy = wall.baseline.end.y - wall.baseline.start.y;
    let dz = wall.baseline.end.z - wall.baseline.start.z;

    if dz.abs() > TOLERANCE {
        return Err(TruckGeometryError::NonHorizontalBaseline);
    }

    let length = (dx * dx + dy * dy).sqrt();
    let along = Vector3::new(dx, dy, 0.0);
    let across = Vector3::new(
        -dy / length * wall.thickness,
        dx / length * wall.thickness,
        0.0,
    );
    let origin = Point3::new(
        wall.baseline.start.x - across.x * 0.5,
        wall.baseline.start.y - across.y * 0.5,
        wall.baseline.start.z,
    );
    Ok((wall, origin, along, across))
}

fn rectangular_wire(points: [Point3; 4]) -> Wire {
    let vertices = points.map(builder::vertex);
    vec![
        builder::line(&vertices[0], &vertices[1]),
        builder::line(&vertices[1], &vertices[2]),
        builder::line(&vertices[2], &vertices[3]),
        builder::line(&vertices[3], &vertices[0]),
    ]
    .into()
}

impl Default for TruckGeometryKernel {
    fn default() -> Self {
        Self {
            tessellation_tolerance: 0.001,
        }
    }
}

impl GeometryKernel for TruckGeometryKernel {
    type Error = TruckGeometryError;

    fn wall_mesh(&self, wall: WallGeometry) -> Result<Mesh, Self::Error> {
        let solid = self.wall_solid(wall)?;
        Ok(self.tessellate(&solid))
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq)]
pub enum TruckGeometryError {
    #[error(transparent)]
    InvalidWall(#[from] rustit_geometry::GeometryError),
    #[error("Truck wall prototype currently requires a horizontal baseline")]
    NonHorizontalBaseline,
    #[error("tessellation tolerance must be finite and greater than Truck's tolerance")]
    InvalidTessellationTolerance,
    #[error("Truck could not complete the solid Boolean operation")]
    BooleanFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustit_geometry::{Segment3, WallGeometry};

    fn wall(length: f64, thickness: f64, height: f64) -> WallGeometry {
        WallGeometry {
            baseline: Segment3::new(
                RustitPoint3::new(1.0, 2.0, 0.0),
                RustitPoint3::new(1.0 + length, 2.0, 0.0),
            ),
            thickness,
            height,
        }
    }

    #[test]
    fn generates_a_closed_truck_solid_and_triangle_mesh() {
        let kernel = TruckGeometryKernel::default();
        let solid = kernel.wall_solid(wall(6.0, 0.2, 3.0)).expect("wall solid");
        let mesh = kernel.wall_mesh(wall(6.0, 0.2, 3.0)).expect("wall mesh");

        assert_eq!(
            solid.boundaries()[0].shell_condition(),
            ShellCondition::Closed
        );
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.triangles.is_empty());
        assert!(
            mesh.triangles
                .iter()
                .flatten()
                .all(|index| (*index as usize) < mesh.vertices.len())
        );
    }

    #[test]
    fn parameter_changes_regenerate_the_mesh_extents() {
        let kernel = TruckGeometryKernel::default();
        let short = kernel.wall_mesh(wall(4.0, 0.2, 3.0)).expect("short wall");
        let tall = kernel.wall_mesh(wall(4.0, 0.2, 4.5)).expect("tall wall");

        assert_eq!(maximum_z(&short), 3.0);
        assert_eq!(maximum_z(&tall), 4.5);
        assert_ne!(maximum_z(&short), maximum_z(&tall));
    }

    #[test]
    fn repeated_generation_is_deterministic() {
        let kernel = TruckGeometryKernel::default();
        let first = kernel.wall_mesh(wall(4.0, 0.2, 3.0)).expect("first mesh");
        let second = kernel.wall_mesh(wall(4.0, 0.2, 3.0)).expect("second mesh");

        assert_eq!(first, second);
    }

    #[test]
    fn creates_a_closed_wall_with_a_rectangular_opening() {
        let kernel = TruckGeometryKernel::default();
        let opening = RectangularOpeningGeometry {
            offset: 1.0,
            sill_height: 0.8,
            width: 1.2,
            height: 1.6,
        };
        let solid = kernel
            .wall_solid_with_opening(wall(5.0, 0.2, 3.0), opening)
            .expect("wall with opening");
        let mesh = kernel.tessellate(&solid);

        assert_eq!(
            solid.boundaries()[0].shell_condition(),
            ShellCondition::Closed
        );
        assert!(mesh.triangles.len() > 12);
        assert!(mesh.vertices.iter().any(|vertex| {
            (vertex.x - 2.0).abs() < TOLERANCE && (vertex.z - 0.8).abs() < TOLERANCE
        }));
    }

    #[test]
    fn cuts_a_rectangular_opening_with_a_boolean() {
        let kernel = TruckGeometryKernel::default();
        let solid = kernel.wall_solid(wall(5.0, 0.2, 3.0)).expect("wall");
        let cutter = WallGeometry {
            baseline: Segment3::new(
                RustitPoint3::new(2.0, 2.0, 0.8),
                RustitPoint3::new(3.2, 2.0, 0.8),
            ),
            thickness: 0.4,
            height: 1.6,
        };
        let cutter = kernel.wall_solid(cutter).expect("opening cutter");
        let opened = kernel.subtract(&solid, &cutter).expect("boolean opening");

        assert_eq!(
            opened.boundaries()[0].shell_condition(),
            ShellCondition::Closed
        );
        assert!(kernel.tessellate(&opened).triangles.len() > 12);
    }

    #[test]
    fn reports_the_current_l_wall_join_limitation() {
        let kernel = TruckGeometryKernel::default();
        let first = kernel.wall_solid(wall(4.0, 0.2, 3.0)).expect("first wall");
        let second_wall = WallGeometry {
            baseline: Segment3::new(
                RustitPoint3::new(1.0, 2.0, 0.0),
                RustitPoint3::new(1.0, 6.0, 0.0),
            ),
            thickness: 0.2,
            height: 3.0,
        };
        let second = kernel.wall_solid(second_wall).expect("second wall");
        assert!(matches!(
            kernel.union(&first, &second),
            Err(TruckGeometryError::BooleanFailed)
        ));
    }

    #[test]
    fn exports_a_modeled_wall_as_step() {
        let kernel = TruckGeometryKernel::default();
        let solid = kernel.wall_solid(wall(4.0, 0.2, 3.0)).expect("wall solid");
        let step = kernel.solid_to_step(&solid);

        assert!(step.starts_with("ISO-10303-21;"));
        assert!(step.contains("MANIFOLD_SOLID_BREP"));
        assert!(step.contains("END-ISO-10303-21;"));
    }

    #[test]
    fn rejects_sloped_wall_baselines_explicitly() {
        let kernel = TruckGeometryKernel::default();
        let sloped = WallGeometry {
            baseline: Segment3::new(
                RustitPoint3::new(0.0, 0.0, 0.0),
                RustitPoint3::new(4.0, 0.0, 1.0),
            ),
            thickness: 0.2,
            height: 3.0,
        };

        assert_eq!(
            kernel.wall_mesh(sloped),
            Err(TruckGeometryError::NonHorizontalBaseline)
        );
    }

    fn maximum_z(mesh: &Mesh) -> f64 {
        mesh.vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f64::NEG_INFINITY, f64::max)
    }
}
