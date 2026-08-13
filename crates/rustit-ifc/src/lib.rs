//! IFC-aligned foundations shared by Rustit's authoring domains.
//!
//! Rustit uses IFC 4.3.2.0 concepts as its semantic baseline while retaining a
//! transaction-friendly Rust model. STEP serialization and 22-character IFC
//! `GlobalId` encoding belong at the interchange boundary.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// IFC schema release targeted by the initial project model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IfcSchemaVersion {
    #[default]
    Ifc4x3Add2,
}

impl IfcSchemaVersion {
    #[must_use]
    pub const fn schema_identifier(self) -> &'static str {
        match self {
            Self::Ifc4x3Add2 => "IFC4X3_ADD2",
        }
    }
}

/// IFC entity families represented in the v0.0.1 project model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcEntity {
    Project,
    BuildingStorey,
    Wall,
    Task,
    RelSequence,
    RelAssignsToProcess,
}

impl IfcEntity {
    #[must_use]
    pub const fn schema_name(self) -> &'static str {
        match self {
            Self::Project => "IfcProject",
            Self::BuildingStorey => "IfcBuildingStorey",
            Self::Wall => "IfcWall",
            Self::Task => "IfcTask",
            Self::RelSequence => "IfcRelSequence",
            Self::RelAssignsToProcess => "IfcRelAssignsToProcess",
        }
    }
}

/// Canonical 128-bit identity for an IFC-rooted Rustit object.
///
/// IFC serializers can losslessly encode this UUID as an `IfcGlobalId`. Rustit
/// keeps the uncompressed value internally to preserve typed, auditable IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IfcRootId(Uuid);

impl IfcRootId {
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

impl Default for IfcRootId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IfcRootId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Known construction classification systems plus an extension point.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassificationSystem {
    MasterFormat,
    UniFormat,
    Other(String),
}

impl ClassificationSystem {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::MasterFormat => "MasterFormat",
            Self::UniFormat => "UniFormat",
            Self::Other(name) => name,
        }
    }
}

/// IFC-aligned reference to a classification code.
///
/// This corresponds to an `IfcClassificationReference` associated to an
/// object. Rustit stores references, not copyrighted classification tables.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClassificationReference {
    pub system: ClassificationSystem,
    pub edition: Option<String>,
    pub identification: String,
    pub name: Option<String>,
}

impl ClassificationReference {
    pub fn new(
        system: ClassificationSystem,
        edition: Option<String>,
        identification: impl Into<String>,
        name: Option<String>,
    ) -> Result<Self, ClassificationError> {
        let identification = identification.into();
        if identification.trim().is_empty() {
            return Err(ClassificationError::EmptyIdentification);
        }

        Ok(Self {
            system,
            edition,
            identification,
            name,
        })
    }

    pub fn master_format(
        edition: impl Into<String>,
        identification: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ClassificationError> {
        Self::new(
            ClassificationSystem::MasterFormat,
            Some(edition.into()),
            identification,
            Some(name.into()),
        )
    }

    pub fn uni_format(
        edition: impl Into<String>,
        identification: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ClassificationError> {
        Self::new(
            ClassificationSystem::UniFormat,
            Some(edition.into()),
            identification,
            Some(name.into()),
        )
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationError {
    #[error("classification identification must not be empty")]
    EmptyIdentification,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_the_official_ifc_4_3_add2_schema_identifier() {
        assert_eq!(
            IfcSchemaVersion::Ifc4x3Add2.schema_identifier(),
            "IFC4X3_ADD2"
        );
    }

    #[test]
    fn retains_classification_system_and_edition() {
        let reference =
            ClassificationReference::uni_format("project edition", "B2010", "Exterior Walls")
                .expect("valid classification reference");

        assert_eq!(reference.system, ClassificationSystem::UniFormat);
        assert_eq!(reference.edition.as_deref(), Some("project edition"));
    }
}
