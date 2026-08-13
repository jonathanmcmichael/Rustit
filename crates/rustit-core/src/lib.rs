//! The unified Rustit project model.
//!
//! BIM authoring and scheduling meet here. Neither domain is owned by a GUI,
//! database, vendor adapter, or AI client.

use rustit_model::{BimModel, ElementId};
use rustit_schedule::{ActivityId, Schedule};
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

stable_id!(ProjectId);
stable_id!(ElementActivityLinkId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementActivityRole {
    Construct,
    Demolish,
    Temporary,
    Inspect,
}

/// Vendor-independent 4D relationship between a BIM element and schedule work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementActivityLink {
    pub id: ElementActivityLinkId,
    pub element_id: ElementId,
    pub activity_id: ActivityId,
    pub role: ElementActivityRole,
}

impl ElementActivityLink {
    #[must_use]
    pub fn new(element_id: ElementId, activity_id: ActivityId, role: ElementActivityRole) -> Self {
        Self {
            id: ElementActivityLinkId::new(),
            element_id,
            activity_id,
            role,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub model: BimModel,
    pub schedule: Schedule,
    pub element_activity_links: Vec<ElementActivityLink>,
}

impl Project {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::new(),
            name: name.into(),
            model: BimModel::default(),
            schedule: Schedule::default(),
            element_activity_links: Vec::new(),
        }
    }

    pub fn link_element_activity(&mut self, link: ElementActivityLink) -> Result<(), ProjectError> {
        if !self.model.contains_element(link.element_id) {
            return Err(ProjectError::UnknownElement(link.element_id));
        }
        if !self.schedule.contains_activity(link.activity_id) {
            return Err(ProjectError::UnknownActivity(link.activity_id));
        }
        if self
            .element_activity_links
            .iter()
            .any(|existing| existing.id == link.id)
        {
            return Err(ProjectError::DuplicateLink(link.id));
        }
        self.element_activity_links.push(link);
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectError {
    #[error("element {0} does not exist")]
    UnknownElement(ElementId),
    #[error("activity {0} does not exist")]
    UnknownActivity(ActivityId),
    #[error("element-activity link {0} already exists")]
    DuplicateLink(ElementActivityLinkId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustit_geometry::{Point3, Segment3};
    use rustit_model::{Level, Wall};
    use rustit_schedule::Activity;

    #[test]
    fn creates_a_vendor_independent_4d_link() {
        let mut project = Project::new("Test project");
        let level = Level::new("Level 1", 0.0);
        let wall = Wall::new(
            level.id,
            Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 0.0, 0.0)),
            0.2,
            3.0,
        )
        .expect("valid wall");
        let activity = Activity::new("Frame wall", 8);
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);

        project.model.add_level(level).expect("add level");
        project.model.add_wall(wall).expect("add wall");
        project
            .schedule
            .add_activity(activity)
            .expect("add activity");
        project
            .link_element_activity(link.clone())
            .expect("link element and activity");

        assert_eq!(project.element_activity_links, vec![link]);
    }
}
