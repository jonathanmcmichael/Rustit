//! Semantic BIM authoring types.

use rustit_geometry::{Segment3, WallGeometry};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id!(ElementId);
stable_id!(LevelId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub id: LevelId,
    pub name: String,
    /// Elevation in metres relative to the project datum.
    pub elevation: f64,
}

impl Level {
    #[must_use]
    pub fn new(name: impl Into<String>, elevation: f64) -> Self {
        Self {
            id: LevelId::new(),
            name: name.into(),
            elevation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub id: ElementId,
    pub level_id: LevelId,
    pub baseline: Segment3,
    /// Wall thickness in metres.
    pub thickness: f64,
    /// Unconnected wall height in metres.
    pub height: f64,
}

impl Wall {
    pub fn new(
        level_id: LevelId,
        baseline: Segment3,
        thickness: f64,
        height: f64,
    ) -> Result<Self, ModelError> {
        WallGeometry {
            baseline,
            thickness,
            height,
        }
        .validate()?;

        Ok(Self {
            id: ElementId::new(),
            level_id,
            baseline,
            thickness,
            height,
        })
    }

    #[must_use]
    pub const fn geometry(&self) -> WallGeometry {
        WallGeometry {
            baseline: self.baseline,
            thickness: self.thickness,
            height: self.height,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BimModel {
    pub levels: Vec<Level>,
    pub walls: Vec<Wall>,
}

impl BimModel {
    pub fn add_level(&mut self, level: Level) -> Result<(), ModelError> {
        if self.levels.iter().any(|existing| existing.id == level.id) {
            return Err(ModelError::DuplicateLevel(level.id));
        }
        self.levels.push(level);
        Ok(())
    }

    pub fn add_wall(&mut self, wall: Wall) -> Result<(), ModelError> {
        if !self.levels.iter().any(|level| level.id == wall.level_id) {
            return Err(ModelError::UnknownLevel(wall.level_id));
        }
        if self.walls.iter().any(|existing| existing.id == wall.id) {
            return Err(ModelError::DuplicateElement(wall.id));
        }
        self.walls.push(wall);
        Ok(())
    }

    #[must_use]
    pub fn contains_element(&self, id: ElementId) -> bool {
        self.walls.iter().any(|wall| wall.id == id)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ModelError {
    #[error(transparent)]
    InvalidGeometry(#[from] rustit_geometry::GeometryError),
    #[error("level {0} does not exist")]
    UnknownLevel(LevelId),
    #[error("level {0} already exists")]
    DuplicateLevel(LevelId),
    #[error("element {0} already exists")]
    DuplicateElement(ElementId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustit_geometry::Point3;

    #[test]
    fn wall_requires_an_existing_level() {
        let level = Level::new("Level 1", 0.0);
        let wall = Wall::new(
            level.id,
            Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 0.0, 0.0)),
            0.2,
            3.0,
        )
        .expect("valid wall");
        let mut model = BimModel::default();

        assert_eq!(
            model.add_wall(wall),
            Err(ModelError::UnknownLevel(level.id))
        );
    }

    #[test]
    fn stable_ids_survive_serialization() {
        let level = Level::new("Level 1", 0.0);
        let json = serde_json::to_string(&level).expect("serialize level");
        let restored: Level = serde_json::from_str(&json).expect("deserialize level");

        assert_eq!(restored.id, level.id);
    }
}
