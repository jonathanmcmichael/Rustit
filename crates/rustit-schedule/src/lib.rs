//! Open schedule types and a deterministic critical path method engine.

use rustit_ifc::{ClassificationReference, IfcEntity, IfcRootId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use uuid::Uuid;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(IfcRootId);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(IfcRootId::new())
            }

            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(IfcRootId::from_uuid(value))
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0.as_uuid()
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

stable_id!(ActivityId);
stable_id!(ActivityRelationshipId);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityStatus {
    #[default]
    NotStarted,
    InProgress,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub id: ActivityId,
    pub name: String,
    /// Planned working duration in hours. Zero represents a milestone.
    pub duration_hours: u32,
    pub status: ActivityStatus,
    pub classifications: Vec<ClassificationReference>,
}

impl Activity {
    #[must_use]
    pub fn new(name: impl Into<String>, duration_hours: u32) -> Self {
        Self {
            id: ActivityId::new(),
            name: name.into(),
            duration_hours,
            status: ActivityStatus::NotStarted,
            classifications: Vec::new(),
        }
    }

    #[must_use]
    pub const fn ifc_entity(&self) -> IfcEntity {
        IfcEntity::Task
    }

    pub fn add_classification(&mut self, reference: ClassificationReference) {
        if !self.classifications.contains(&reference) {
            self.classifications.push(reference);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    FinishStart,
    StartStart,
    FinishFinish,
    StartFinish,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityRelationship {
    pub id: ActivityRelationshipId,
    pub predecessor_id: ActivityId,
    pub successor_id: ActivityId,
    pub relationship_type: RelationshipType,
    /// Working-hour offset. Negative values represent lead.
    pub lag_hours: i32,
}

impl ActivityRelationship {
    #[must_use]
    pub fn new(
        predecessor_id: ActivityId,
        successor_id: ActivityId,
        relationship_type: RelationshipType,
        lag_hours: i32,
    ) -> Self {
        Self {
            id: ActivityRelationshipId::new(),
            predecessor_id,
            successor_id,
            relationship_type,
            lag_hours,
        }
    }

    #[must_use]
    pub const fn ifc_entity(&self) -> IfcEntity {
        IfcEntity::RelSequence
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub activities: Vec<Activity>,
    pub relationships: Vec<ActivityRelationship>,
}

impl Schedule {
    pub fn add_activity(&mut self, activity: Activity) -> Result<(), ScheduleError> {
        if self.contains_activity(activity.id) {
            return Err(ScheduleError::DuplicateActivity(activity.id));
        }
        self.activities.push(activity);
        Ok(())
    }

    pub fn add_relationship(
        &mut self,
        relationship: ActivityRelationship,
    ) -> Result<(), ScheduleError> {
        if self
            .relationships
            .iter()
            .any(|existing| existing.id == relationship.id)
        {
            return Err(ScheduleError::DuplicateRelationship(relationship.id));
        }
        if !self.contains_activity(relationship.predecessor_id) {
            return Err(ScheduleError::UnknownActivity(relationship.predecessor_id));
        }
        if !self.contains_activity(relationship.successor_id) {
            return Err(ScheduleError::UnknownActivity(relationship.successor_id));
        }
        self.relationships.push(relationship);
        Ok(())
    }

    #[must_use]
    pub fn contains_activity(&self, id: ActivityId) -> bool {
        self.activities.iter().any(|activity| activity.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpmTiming {
    pub activity_id: ActivityId,
    pub early_start: i64,
    pub early_finish: i64,
    pub late_start: i64,
    pub late_finish: i64,
    pub total_float: i64,
    pub is_critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpmResult {
    pub project_duration_hours: i64,
    pub timings: Vec<CpmTiming>,
}

/// Computes an unconstrained CPM network in working-hour units.
///
/// Calendar expansion and date anchoring intentionally remain outside v0.0.1.
pub fn calculate_cpm(schedule: &Schedule) -> Result<CpmResult, ScheduleError> {
    let mut activities = HashMap::with_capacity(schedule.activities.len());
    let mut indegree = HashMap::with_capacity(schedule.activities.len());
    let mut outgoing: HashMap<ActivityId, Vec<(ActivityId, i64)>> = HashMap::new();

    for activity in &schedule.activities {
        if activities.insert(activity.id, activity).is_some() {
            return Err(ScheduleError::DuplicateActivity(activity.id));
        }
        indegree.insert(activity.id, 0_usize);
    }

    for relationship in &schedule.relationships {
        let predecessor = activities
            .get(&relationship.predecessor_id)
            .ok_or(ScheduleError::UnknownActivity(relationship.predecessor_id))?;
        let successor = activities
            .get(&relationship.successor_id)
            .ok_or(ScheduleError::UnknownActivity(relationship.successor_id))?;
        let weight = relationship_weight(predecessor, successor, relationship);

        outgoing
            .entry(relationship.predecessor_id)
            .or_default()
            .push((relationship.successor_id, weight));
        *indegree
            .get_mut(&relationship.successor_id)
            .expect("validated activity") += 1;
    }

    let mut ready: VecDeque<ActivityId> = indegree
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    let mut order = Vec::with_capacity(activities.len());
    let mut earliest: HashMap<ActivityId, i64> = activities.keys().map(|id| (*id, 0_i64)).collect();

    while let Some(activity_id) = ready.pop_front() {
        order.push(activity_id);
        let current_start = earliest[&activity_id];

        for (successor_id, weight) in outgoing.get(&activity_id).into_iter().flatten() {
            let candidate = current_start + weight;
            let successor_start = earliest.get_mut(successor_id).expect("validated successor");
            *successor_start = (*successor_start).max(candidate);

            let count = indegree.get_mut(successor_id).expect("validated successor");
            *count -= 1;
            if *count == 0 {
                ready.push_back(*successor_id);
            }
        }
    }

    if order.len() != activities.len() {
        return Err(ScheduleError::CyclicNetwork);
    }

    let project_duration_hours = order
        .iter()
        .map(|id| earliest[id] + i64::from(activities[id].duration_hours))
        .max()
        .unwrap_or_default();
    let mut latest: HashMap<ActivityId, i64> = activities
        .iter()
        .map(|(id, activity)| {
            (
                *id,
                project_duration_hours - i64::from(activity.duration_hours),
            )
        })
        .collect();

    for activity_id in order.iter().rev() {
        if let Some(edges) = outgoing.get(activity_id) {
            for (successor_id, weight) in edges {
                let candidate = latest[successor_id] - weight;
                let activity_start = latest.get_mut(activity_id).expect("known activity");
                *activity_start = (*activity_start).min(candidate);
            }
        }
    }

    let timings = schedule
        .activities
        .iter()
        .map(|activity| {
            let early_start = earliest[&activity.id];
            let late_start = latest[&activity.id];
            let duration = i64::from(activity.duration_hours);
            let total_float = late_start - early_start;
            CpmTiming {
                activity_id: activity.id,
                early_start,
                early_finish: early_start + duration,
                late_start,
                late_finish: late_start + duration,
                total_float,
                is_critical: total_float == 0,
            }
        })
        .collect();

    Ok(CpmResult {
        project_duration_hours,
        timings,
    })
}

fn relationship_weight(
    predecessor: &Activity,
    successor: &Activity,
    relationship: &ActivityRelationship,
) -> i64 {
    let predecessor_duration = i64::from(predecessor.duration_hours);
    let successor_duration = i64::from(successor.duration_hours);
    let lag = i64::from(relationship.lag_hours);

    match relationship.relationship_type {
        RelationshipType::FinishStart => predecessor_duration + lag,
        RelationshipType::StartStart => lag,
        RelationshipType::FinishFinish => predecessor_duration + lag - successor_duration,
        RelationshipType::StartFinish => lag - successor_duration,
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScheduleError {
    #[error("activity {0} already exists")]
    DuplicateActivity(ActivityId),
    #[error("activity relationship {0} already exists")]
    DuplicateRelationship(ActivityRelationshipId),
    #[error("activity {0} does not exist")]
    UnknownActivity(ActivityId),
    #[error("activity network contains a cycle")]
    CyclicNetwork,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_a_finish_to_start_critical_path() {
        let first = Activity::new("Lay out wall", 8);
        let second = Activity::new("Frame wall", 4);
        let relationship =
            ActivityRelationship::new(first.id, second.id, RelationshipType::FinishStart, 0);
        let schedule = Schedule {
            activities: vec![first, second],
            relationships: vec![relationship],
        };

        let result = calculate_cpm(&schedule).expect("valid network");

        assert_eq!(result.project_duration_hours, 12);
        assert!(result.timings.iter().all(|timing| timing.is_critical));
    }

    #[test]
    fn reports_float_for_a_short_independent_activity() {
        let first = Activity::new("A", 8);
        let second = Activity::new("B", 4);
        let independent = Activity::new("C", 2);
        let relationship =
            ActivityRelationship::new(first.id, second.id, RelationshipType::FinishStart, 0);
        let schedule = Schedule {
            activities: vec![first, second, independent.clone()],
            relationships: vec![relationship],
        };

        let result = calculate_cpm(&schedule).expect("valid network");
        let timing = result
            .timings
            .iter()
            .find(|timing| timing.activity_id == independent.id)
            .expect("independent timing");

        assert_eq!(timing.total_float, 10);
        assert!(!timing.is_critical);
    }

    #[test]
    fn rejects_cycles() {
        let first = Activity::new("A", 8);
        let second = Activity::new("B", 4);
        let schedule = Schedule {
            activities: vec![first.clone(), second.clone()],
            relationships: vec![
                ActivityRelationship::new(first.id, second.id, RelationshipType::FinishStart, 0),
                ActivityRelationship::new(second.id, first.id, RelationshipType::FinishStart, 0),
            ],
        };

        assert_eq!(calculate_cpm(&schedule), Err(ScheduleError::CyclicNetwork));
    }

    #[test]
    fn stable_ids_survive_serialization() {
        let activity = Activity::new("Frame wall", 8);
        let json = serde_json::to_string(&activity).expect("serialize activity");
        let restored: Activity = serde_json::from_str(&json).expect("deserialize activity");

        assert_eq!(restored.id, activity.id);
    }
}
