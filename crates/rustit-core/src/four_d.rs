//! Deterministic 4D state of the building model at a point in schedule time.
//!
//! 4D is derived, never authored: the semantic model, the schedule, and the
//! [`ElementActivityLink`]s remain the source of truth. A [`FourDTimeline`] is
//! built once from a calculated CPM result and can then be queried repeatedly
//! (for playback or scrubbing) without recalculating the schedule.
//!
//! Time is expressed in the same working-hour offsets as the CPM engine. No
//! calendar or date semantics are inferred.
//!
//! # Interval convention
//!
//! A linked activity occupies the half-open interval `[start, finish)`. At
//! `hour == start` the work has begun; at `hour == finish` it is complete. A
//! zero-duration milestone therefore completes instantly and is never reported
//! as an active link.
//!
//! # Presence rules
//!
//! [`ElementActivityRole::Construct`], [`ElementActivityRole::Demolish`], and
//! [`ElementActivityRole::Temporary`] affect whether an element exists.
//! [`ElementActivityRole::Inspect`] never changes existence; it only appears in
//! [`ElementState::active_links`].
//!
//! When an element has several existence-affecting links, the link with the
//! latest start at or before the query hour governs its presence. Before any of
//! them has started, the earliest one governs. Two existence-affecting links
//! on the same element may not share a start hour, because neither could be
//! said to govern; [`FourDTimeline::build`] rejects that case.

use crate::{ElementActivityLinkId, ElementActivityRole, Project};
use rustit_model::ElementId;
use rustit_schedule::{ActivityId, CpmResult, CpmTiming};
use std::collections::HashMap;
use thiserror::Error;

/// Which CPM dates position linked work on the 4D timeline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DateBasis {
    /// Early start and early finish.
    #[default]
    Early,
    /// Late start and late finish.
    Late,
}

/// Derived existence state of one element at one hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementPresence {
    /// No construct, demolish, or temporary link positions this element in time.
    Unscheduled,
    /// Permanent or temporary work that has not started.
    Planned,
    /// Construct work is in progress.
    UnderConstruction,
    /// Construct work is complete.
    Built,
    /// The element exists before its demolition starts.
    Existing,
    /// Demolish work is in progress.
    UnderDemolition,
    /// Temporary work is in place.
    Temporary,
    /// Demolition is complete or temporary work has been removed.
    Removed,
}

/// 4D state of one element at one hour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementState {
    pub element_id: ElementId,
    pub presence: ElementPresence,
    /// Links of any role whose activity is in progress at the query hour, in
    /// project link order.
    pub active_links: Vec<ElementActivityLinkId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LinkInterval {
    link_id: ElementActivityLinkId,
    role: ElementActivityRole,
    start: i64,
    finish: i64,
}

impl LinkInterval {
    const fn is_active_at(&self, hour: i64) -> bool {
        self.start <= hour && hour < self.finish
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementTrack {
    element_id: ElementId,
    /// Existence-affecting intervals sorted by strictly increasing start.
    existence: Vec<LinkInterval>,
    /// Every linked interval in project link order.
    links: Vec<LinkInterval>,
}

impl ElementTrack {
    fn presence_at(&self, hour: i64) -> ElementPresence {
        let started = self
            .existence
            .partition_point(|interval| interval.start <= hour);
        let governing = match started {
            0 => match self.existence.first() {
                Some(first) => first,
                None => return ElementPresence::Unscheduled,
            },
            count => &self.existence[count - 1],
        };

        let phase = if hour < governing.start {
            Phase::Before
        } else if hour < governing.finish {
            Phase::During
        } else {
            Phase::After
        };

        match (governing.role, phase) {
            (ElementActivityRole::Construct, Phase::Before)
            | (ElementActivityRole::Temporary, Phase::Before) => ElementPresence::Planned,
            (ElementActivityRole::Construct, Phase::During) => ElementPresence::UnderConstruction,
            (ElementActivityRole::Construct, Phase::After) => ElementPresence::Built,
            (ElementActivityRole::Demolish, Phase::Before) => ElementPresence::Existing,
            (ElementActivityRole::Demolish, Phase::During) => ElementPresence::UnderDemolition,
            (ElementActivityRole::Temporary, Phase::During) => ElementPresence::Temporary,
            (ElementActivityRole::Demolish, Phase::After)
            | (ElementActivityRole::Temporary, Phase::After) => ElementPresence::Removed,
            (ElementActivityRole::Inspect, _) => {
                unreachable!("inspect links never enter the existence track")
            }
        }
    }

    fn state_at(&self, hour: i64) -> ElementState {
        ElementState {
            element_id: self.element_id,
            presence: self.presence_at(hour),
            active_links: self
                .links
                .iter()
                .filter(|interval| interval.is_active_at(hour))
                .map(|interval| interval.link_id)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Before,
    During,
    After,
}

/// Precomputed 4D timeline for every element in a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourDTimeline {
    basis: DateBasis,
    tracks: Vec<ElementTrack>,
    change_hours: Vec<i64>,
}

impl FourDTimeline {
    /// Builds a timeline from a project and a CPM result calculated for its
    /// schedule.
    ///
    /// Every model element receives a track, in model order, whether or not it
    /// is linked to schedule work.
    pub fn build(project: &Project, cpm: &CpmResult, basis: DateBasis) -> Result<Self, FourDError> {
        let timings: HashMap<ActivityId, &CpmTiming> = cpm
            .timings
            .iter()
            .map(|timing| (timing.activity_id, timing))
            .collect();

        let mut tracks: Vec<ElementTrack> = project
            .model
            .walls
            .iter()
            .map(|wall| ElementTrack {
                element_id: wall.id,
                existence: Vec::new(),
                links: Vec::new(),
            })
            .collect();
        let track_index: HashMap<ElementId, usize> = tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track.element_id, index))
            .collect();
        let mut change_hours = Vec::with_capacity(project.element_activity_links.len() * 2);

        for link in &project.element_activity_links {
            let index = *track_index
                .get(&link.element_id)
                .ok_or(FourDError::UnknownElement(link.element_id))?;
            let timing = timings
                .get(&link.activity_id)
                .ok_or(FourDError::MissingTiming(link.activity_id))?;
            let (start, finish) = match basis {
                DateBasis::Early => (timing.early_start, timing.early_finish),
                DateBasis::Late => (timing.late_start, timing.late_finish),
            };
            let interval = LinkInterval {
                link_id: link.id,
                role: link.role,
                start,
                finish,
            };

            let track = &mut tracks[index];
            track.links.push(interval);
            if link.role != ElementActivityRole::Inspect {
                track.existence.push(interval);
            }
            change_hours.extend([start, finish]);
        }

        for track in &mut tracks {
            track.existence.sort_by_key(|interval| interval.start);
            if let Some(pair) = track
                .existence
                .windows(2)
                .find(|pair| pair[0].start == pair[1].start)
            {
                return Err(FourDError::SimultaneousExistenceLinks {
                    element_id: track.element_id,
                    hour: pair[0].start,
                });
            }
        }

        change_hours.sort_unstable();
        change_hours.dedup();

        Ok(Self {
            basis,
            tracks,
            change_hours,
        })
    }

    #[must_use]
    pub const fn basis(&self) -> DateBasis {
        self.basis
    }

    /// Sorted, distinct hours at which any element's state can change.
    ///
    /// Playback only needs to redraw at these hours.
    #[must_use]
    pub fn change_hours(&self) -> &[i64] {
        &self.change_hours
    }

    /// State of every model element at `hour`, in model order.
    #[must_use]
    pub fn state_at(&self, hour: i64) -> Vec<ElementState> {
        self.tracks
            .iter()
            .map(|track| track.state_at(hour))
            .collect()
    }

    /// State of one element at `hour`, or `None` if it is not in the model.
    #[must_use]
    pub fn element_state_at(&self, element_id: ElementId, hour: i64) -> Option<ElementState> {
        self.tracks
            .iter()
            .find(|track| track.element_id == element_id)
            .map(|track| track.state_at(hour))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FourDError {
    #[error("element-activity link references element {0}, which is not in the model")]
    UnknownElement(ElementId),
    #[error("CPM result has no timing for linked activity {0}")]
    MissingTiming(ActivityId),
    #[error(
        "element {element_id} has more than one construct, demolish, or temporary link starting at hour {hour}"
    )]
    SimultaneousExistenceLinks { element_id: ElementId, hour: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementActivityLink;
    use rustit_geometry::{Point3, Segment3};
    use rustit_model::{Level, Wall};
    use rustit_schedule::{Activity, ActivityRelationship, RelationshipType, calculate_cpm};

    fn project_with_walls(count: usize) -> (Project, Vec<ElementId>) {
        let mut project = Project::new("Dungeon Floor 1 4D unit test");
        let level = Level::new("Dungeon Floor 1", 0.0);
        let level_id = level.id;
        project.model.add_level(level).expect("add level");
        let ids = (0..count)
            .map(|offset| {
                let y = offset as f64;
                let wall = Wall::new(
                    level_id,
                    Segment3::new(Point3::new(0.0, y, 0.0), Point3::new(4.0, y, 0.0)),
                    0.2,
                    3.0,
                )
                .expect("valid wall");
                let id = wall.id;
                project.model.add_wall(wall).expect("add wall");
                id
            })
            .collect();
        (project, ids)
    }

    fn link(
        project: &mut Project,
        element_id: ElementId,
        activity: &Activity,
        role: ElementActivityRole,
    ) -> ElementActivityLinkId {
        let link = ElementActivityLink::new(element_id, activity.id, role);
        let id = link.id;
        project.link_element_activity(link).expect("valid link");
        id
    }

    fn presence(timeline: &FourDTimeline, element_id: ElementId, hour: i64) -> ElementPresence {
        timeline
            .element_state_at(element_id, hour)
            .expect("element in model")
            .presence
    }

    /// A wall framed in hours 0–8 and demolished in hours 12–16 walks through
    /// every permanent-work state, with the later-starting link governing.
    #[test]
    fn construct_then_demolish_walks_the_full_lifecycle() {
        let (mut project, walls) = project_with_walls(1);
        let build = Activity::new("Carl frames wall", 8);
        let demolish = Activity::new("Mongo demolishes wall", 4);
        project.schedule.add_activity(build.clone()).expect("add");
        project
            .schedule
            .add_activity(demolish.clone())
            .expect("add");
        project
            .schedule
            .add_relationship(ActivityRelationship::new(
                build.id,
                demolish.id,
                RelationshipType::FinishStart,
                4,
            ))
            .expect("relationship");
        link(
            &mut project,
            walls[0],
            &build,
            ElementActivityRole::Construct,
        );
        link(
            &mut project,
            walls[0],
            &demolish,
            ElementActivityRole::Demolish,
        );

        let cpm = calculate_cpm(&project.schedule).expect("valid network");
        let timeline = FourDTimeline::build(&project, &cpm, DateBasis::Early).expect("timeline");

        assert_eq!(presence(&timeline, walls[0], -1), ElementPresence::Planned);
        assert_eq!(
            presence(&timeline, walls[0], 0),
            ElementPresence::UnderConstruction
        );
        assert_eq!(presence(&timeline, walls[0], 8), ElementPresence::Built);
        assert_eq!(presence(&timeline, walls[0], 11), ElementPresence::Built);
        assert_eq!(
            presence(&timeline, walls[0], 12),
            ElementPresence::UnderDemolition
        );
        assert_eq!(presence(&timeline, walls[0], 16), ElementPresence::Removed);
        assert_eq!(timeline.change_hours(), &[0, 8, 12, 16]);
    }

    #[test]
    fn a_milestone_construct_link_completes_instantly_and_is_never_active() {
        let (mut project, walls) = project_with_walls(1);
        let milestone = Activity::new("Wall accepted by Mordecai", 0);
        project
            .schedule
            .add_activity(milestone.clone())
            .expect("add");
        link(
            &mut project,
            walls[0],
            &milestone,
            ElementActivityRole::Construct,
        );

        let cpm = calculate_cpm(&project.schedule).expect("valid network");
        let timeline = FourDTimeline::build(&project, &cpm, DateBasis::Early).expect("timeline");
        let state = timeline.element_state_at(walls[0], 0).expect("element");

        assert_eq!(state.presence, ElementPresence::Built);
        assert!(state.active_links.is_empty());
        assert_eq!(timeline.change_hours(), &[0]);
    }

    #[test]
    fn inspect_links_are_active_without_changing_presence() {
        let (mut project, walls) = project_with_walls(2);
        let inspect = Activity::new("Inspect wall", 4);
        project.schedule.add_activity(inspect.clone()).expect("add");
        let inspect_link = link(
            &mut project,
            walls[0],
            &inspect,
            ElementActivityRole::Inspect,
        );

        let cpm = calculate_cpm(&project.schedule).expect("valid network");
        let timeline = FourDTimeline::build(&project, &cpm, DateBasis::Early).expect("timeline");
        let states = timeline.state_at(2);

        assert_eq!(states.len(), 2);
        assert_eq!(states[0].presence, ElementPresence::Unscheduled);
        assert_eq!(states[0].active_links, vec![inspect_link]);
        assert_eq!(states[1].presence, ElementPresence::Unscheduled);
        assert!(states[1].active_links.is_empty());
    }

    /// `Short` 4 h runs alongside `Long` 12 h with 8 h of float. On a late
    /// basis the wall linked to `Short` is still planned at hour 4 and is built
    /// at hours 8–12 instead of 0–4.
    #[test]
    fn late_basis_uses_late_dates() {
        let (mut project, walls) = project_with_walls(1);
        let long = Activity::new("Long", 12);
        let short = Activity::new("Short", 4);
        project.schedule.add_activity(long).expect("add");
        project.schedule.add_activity(short.clone()).expect("add");
        link(
            &mut project,
            walls[0],
            &short,
            ElementActivityRole::Construct,
        );

        let cpm = calculate_cpm(&project.schedule).expect("valid network");
        let early = FourDTimeline::build(&project, &cpm, DateBasis::Early).expect("timeline");
        let late = FourDTimeline::build(&project, &cpm, DateBasis::Late).expect("timeline");

        assert_eq!(late.basis(), DateBasis::Late);
        assert_eq!(presence(&early, walls[0], 4), ElementPresence::Built);
        assert_eq!(presence(&late, walls[0], 4), ElementPresence::Planned);
        assert_eq!(
            presence(&late, walls[0], 8),
            ElementPresence::UnderConstruction
        );
        assert_eq!(late.change_hours(), &[8, 12]);
    }

    #[test]
    fn rejects_existence_links_that_start_together() {
        let (mut project, walls) = project_with_walls(1);
        let build = Activity::new("Build", 8);
        let shore = Activity::new("Shore", 4);
        project.schedule.add_activity(build.clone()).expect("add");
        project.schedule.add_activity(shore.clone()).expect("add");
        link(
            &mut project,
            walls[0],
            &build,
            ElementActivityRole::Construct,
        );
        link(
            &mut project,
            walls[0],
            &shore,
            ElementActivityRole::Temporary,
        );

        let cpm = calculate_cpm(&project.schedule).expect("valid network");

        assert_eq!(
            FourDTimeline::build(&project, &cpm, DateBasis::Early),
            Err(FourDError::SimultaneousExistenceLinks {
                element_id: walls[0],
                hour: 0,
            })
        );
    }

    #[test]
    fn rejects_a_cpm_result_missing_a_linked_activity() {
        let (mut project, walls) = project_with_walls(1);
        let build = Activity::new("Build", 8);
        project.schedule.add_activity(build.clone()).expect("add");
        link(
            &mut project,
            walls[0],
            &build,
            ElementActivityRole::Construct,
        );
        let stale = CpmResult {
            project_duration_hours: 0,
            timings: Vec::new(),
        };

        assert_eq!(
            FourDTimeline::build(&project, &stale, DateBasis::Early),
            Err(FourDError::MissingTiming(build.id))
        );
    }

    #[test]
    fn rejects_a_link_to_an_element_outside_the_model() {
        let (mut project, _) = project_with_walls(0);
        let build = Activity::new("Build", 8);
        project.schedule.add_activity(build.clone()).expect("add");
        let orphan = ElementId::new();
        // Fields are public, so a deserialized project can bypass
        // `link_element_activity`; the timeline must still refuse it.
        project
            .element_activity_links
            .push(ElementActivityLink::new(
                orphan,
                build.id,
                ElementActivityRole::Construct,
            ));
        let cpm = calculate_cpm(&project.schedule).expect("valid network");

        assert_eq!(
            FourDTimeline::build(&project, &cpm, DateBasis::Early),
            Err(FourDError::UnknownElement(orphan))
        );
    }
}
