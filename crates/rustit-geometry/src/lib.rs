//! Geometry primitives and kernel contracts.
//!
//! This crate deliberately contains no BIM semantics. A future geometry kernel
//! can implement these contracts without becoming the Rustit object model.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A point in project coordinates, measured in metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn distance_to(self, other: Self) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dz = other.z - self.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A directed line segment in project coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Segment3 {
    pub start: Point3,
    pub end: Point3,
}

impl Segment3 {
    #[must_use]
    pub const fn new(start: Point3, end: Point3) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn length(self) -> f64 {
        self.start.distance_to(self.end)
    }
}

/// A triangle mesh suitable for rendering or downstream tessellation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Geometry inputs required to generate a straight wall.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WallGeometry {
    pub baseline: Segment3,
    pub thickness: f64,
    pub height: f64,
}

/// A rectangular through-opening located in a straight wall.
///
/// `offset` is measured from the wall baseline start to the opening's near
/// jamb. `sill_height` is measured from the wall baseline elevation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RectangularOpeningGeometry {
    pub offset: f64,
    pub sill_height: f64,
    pub width: f64,
    pub height: f64,
}

impl RectangularOpeningGeometry {
    pub fn validate_for(self, wall: WallGeometry) -> Result<Self, GeometryError> {
        let wall = wall.validate()?;
        if self.width <= 0.0 {
            return Err(GeometryError::NonPositiveOpeningWidth);
        }
        if self.height <= 0.0 {
            return Err(GeometryError::NonPositiveOpeningHeight);
        }
        if self.offset < 0.0
            || self.sill_height < 0.0
            || self.offset + self.width > wall.baseline.length()
            || self.sill_height + self.height > wall.height
        {
            return Err(GeometryError::OpeningOutsideWall);
        }
        Ok(self)
    }
}

impl WallGeometry {
    pub fn validate(self) -> Result<Self, GeometryError> {
        if self.baseline.length() <= f64::EPSILON {
            return Err(GeometryError::DegenerateBaseline);
        }
        if self.thickness <= 0.0 {
            return Err(GeometryError::NonPositiveThickness);
        }
        if self.height <= 0.0 {
            return Err(GeometryError::NonPositiveHeight);
        }
        Ok(self)
    }
}

/// Boundary implemented by Truck, Open CASCADE, or another geometry provider.
pub trait GeometryKernel {
    type Error;

    fn wall_mesh(&self, wall: WallGeometry) -> Result<Mesh, Self::Error>;
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    #[error("wall baseline must have non-zero length")]
    DegenerateBaseline,
    #[error("wall thickness must be positive")]
    NonPositiveThickness,
    #[error("wall height must be positive")]
    NonPositiveHeight,
    #[error("opening width must be positive")]
    NonPositiveOpeningWidth,
    #[error("opening height must be positive")]
    NonPositiveOpeningHeight,
    #[error("opening must fit within the wall elevation and baseline extents")]
    OpeningOutsideWall,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_wall_geometry() {
        let geometry = WallGeometry {
            baseline: Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 0.0, 0.0)),
            thickness: 0.2,
            height: 3.0,
        };

        assert!(geometry.validate().is_ok());
    }

    #[test]
    fn rejects_degenerate_baseline() {
        let origin = Point3::new(0.0, 0.0, 0.0);
        let geometry = WallGeometry {
            baseline: Segment3::new(origin, origin),
            thickness: 0.2,
            height: 3.0,
        };

        assert_eq!(geometry.validate(), Err(GeometryError::DegenerateBaseline));
    }

    #[test]
    fn validates_an_opening_against_its_wall() {
        let wall = WallGeometry {
            baseline: Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 0.0, 0.0)),
            thickness: 0.2,
            height: 3.0,
        };
        let opening = RectangularOpeningGeometry {
            offset: 1.0,
            sill_height: 0.9,
            width: 1.2,
            height: 1.5,
        };

        assert!(opening.validate_for(wall).is_ok());
        assert_eq!(
            RectangularOpeningGeometry {
                offset: 4.0,
                ..opening
            }
            .validate_for(wall),
            Err(GeometryError::OpeningOutsideWall)
        );
    }
}
