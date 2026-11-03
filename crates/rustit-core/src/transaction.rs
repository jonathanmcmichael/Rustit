//! Typed transactions, per-object versions, and the change log (RFC 0004).
//!
//! Every change to a project is an ordered list of intent-level
//! [`Operation`]s, proposed by an [`Actor`] with a rationale. A
//! [`ProjectSnapshot`] validates the whole proposal against a working copy and
//! returns a [`Preview`]: object-level before/after states plus the base
//! version of each touched object. Nothing is written until
//! [`ProjectSnapshot::commit`] confirms that every touched object is still at
//! its base version and every referenced object still exists. A successful
//! commit appends a [`LogEntry`] at the next [`LogSequence`].
//!
//! Model, schedule, and 4D objects carry independent versions, so edits that
//! touch disjoint objects never conflict.

use crate::{ElementActivityLink, ElementActivityLinkId, Project, ProjectError, ProjectId};
use rustit_geometry::Segment3;
use rustit_model::{ElementId, Level, LevelId, ModelError, Wall};
use rustit_schedule::{
    Activity, ActivityId, ActivityRelationship, ActivityRelationshipId, ScheduleError,
    calculate_cpm,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

/// Per-object version. Starts at 1 on creation and increases by one on every
/// committed change, including deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectVersion(pub u64);

/// Position in a project's totally ordered change log. The genesis snapshot
/// is at sequence 0; the first commit is sequence 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogSequence(pub u64);

/// Stable identity of one proposed or committed transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionId(Uuid);

impl TransactionId {
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

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed reference to one versioned canonical object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectRef {
    Project(ProjectId),
    Level(LevelId),
    Wall(ElementId),
    Activity(ActivityId),
    Relationship(ActivityRelationshipId),
    Link(ElementActivityLinkId),
}

impl ObjectRef {
    /// The project domain this object belongs to.
    #[must_use]
    pub const fn domain(self) -> Domain {
        match self {
            Self::Project(_) => Domain::Project,
            Self::Level(_) | Self::Wall(_) => Domain::Model,
            Self::Activity(_) | Self::Relationship(_) => Domain::Schedule,
            Self::Link(_) => Domain::FourD,
        }
    }

    const fn sort_key(self) -> (u8, Uuid) {
        match self {
            Self::Project(id) => (0, id.as_uuid()),
            Self::Level(id) => (1, id.as_uuid()),
            Self::Wall(id) => (2, id.as_uuid()),
            Self::Activity(id) => (3, id.as_uuid()),
            Self::Relationship(id) => (4, id.as_uuid()),
            Self::Link(id) => (5, id.as_uuid()),
        }
    }
}

/// Deterministic order: by object kind, then by UUID.
impl Ord for ObjectRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for ObjectRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for ObjectRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, id) = match self {
            Self::Project(id) => ("project", id.as_uuid()),
            Self::Level(id) => ("level", id.as_uuid()),
            Self::Wall(id) => ("wall", id.as_uuid()),
            Self::Activity(id) => ("activity", id.as_uuid()),
            Self::Relationship(id) => ("activity relationship", id.as_uuid()),
            Self::Link(id) => ("element-activity link", id.as_uuid()),
        };
        write!(formatter, "{kind} {id}")
    }
}

/// Project domain touched by a change, used to filter the change log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Domain {
    Project,
    Model,
    Schedule,
    FourD,
}

/// Complete state of one versioned object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectState {
    Project { name: String },
    Level(Level),
    Wall(Wall),
    Activity(Activity),
    Relationship(ActivityRelationship),
    Link(ElementActivityLink),
}

/// An intent on stable IDs. Creation operations carry caller-generated IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    RenameProject { name: String },

    CreateLevel(Level),
    RenameLevel { id: LevelId, name: String },
    DeleteLevel { id: LevelId },
    CreateWall(Wall),
    SetWallHeight { id: ElementId, height: f64 },
    SetWallBaseline { id: ElementId, baseline: Segment3 },
    DeleteWall { id: ElementId },

    CreateActivity(Activity),
    SetActivityDuration { id: ActivityId, duration_hours: u32 },
    DeleteActivity { id: ActivityId },
    CreateRelationship(ActivityRelationship),
    DeleteRelationship { id: ActivityRelationshipId },

    LinkElementActivity(ElementActivityLink),
    UnlinkElementActivity { id: ElementActivityLinkId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActorKind {
    Human,
    Integration,
    Agent,
}

/// Who proposed a transaction. Recorded for audit; not an authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub kind: ActorKind,
    /// Opaque, non-secret actor identifier.
    pub id: String,
}

impl Actor {
    #[must_use]
    pub fn new(kind: ActorKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }
}

/// Ordered operations proposed by one actor, not yet validated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: TransactionId,
    pub actor: Actor,
    pub rationale: String,
    pub operations: Vec<Operation>,
}

impl Proposal {
    #[must_use]
    pub fn new(actor: Actor, rationale: impl Into<String>, operations: Vec<Operation>) -> Self {
        Self {
            id: TransactionId::new(),
            actor,
            rationale: rationale.into(),
            operations,
        }
    }
}

/// Effect of a transaction on one object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectChange {
    pub object: ObjectRef,
    /// Version the change was validated against; `None` means the object must
    /// not exist yet.
    pub base_version: Option<ObjectVersion>,
    /// `None` when the transaction creates the object.
    pub before: Option<ObjectState>,
    /// `None` when the transaction deletes the object.
    pub after: Option<ObjectState>,
}

/// Non-blocking finding reported alongside a valid preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Diagnostic {
    /// The schedule after this transaction contains cycles, so CPM cannot be
    /// calculated until they are resolved.
    CyclicSchedule { cycles: Vec<Vec<ActivityId>> },
}

/// A validated proposal: its effects and diagnostics. Nothing is written.
///
/// A preview can only be produced by [`ProjectSnapshot::preview`], so a commit
/// always applies a change set that passed validation.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    project_id: ProjectId,
    proposal: Proposal,
    changes: Vec<ObjectChange>,
    references: Vec<ObjectRef>,
    diagnostics: Vec<Diagnostic>,
}

impl Preview {
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    #[must_use]
    pub const fn proposal(&self) -> &Proposal {
        &self.proposal
    }

    /// Every object whose state changes, once each, in first-touch order.
    #[must_use]
    pub fn changes(&self) -> &[ObjectChange] {
        &self.changes
    }

    /// Objects the transaction relies on without changing, in deterministic
    /// order. Each must still exist at commit.
    #[must_use]
    pub fn references(&self) -> &[ObjectRef] {
        &self.references
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// A committed transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub sequence: LogSequence,
    pub transaction_id: TransactionId,
    pub actor: Actor,
    pub rationale: String,
    pub operations: Vec<Operation>,
    pub changes: Vec<ObjectChange>,
    /// Distinct domains touched by `changes`, in order.
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjectRecord {
    version: ObjectVersion,
    deleted: bool,
}

/// Current project state with every object's version and the log head.
///
/// Deleted objects remain as tombstones so their IDs are never reused.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSnapshot {
    project: Project,
    records: BTreeMap<ObjectRef, ObjectRecord>,
    head: LogSequence,
}

impl ProjectSnapshot {
    /// Starts a change history from an existing project. Every object in it,
    /// including the project itself, begins at version 1 and the head is 0.
    #[must_use]
    pub fn genesis(project: Project) -> Self {
        let records = all_objects(&project)
            .into_iter()
            .map(|object| {
                (
                    object,
                    ObjectRecord {
                        version: ObjectVersion(1),
                        deleted: false,
                    },
                )
            })
            .collect();
        Self {
            project,
            records,
            head: LogSequence(0),
        }
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub const fn head(&self) -> LogSequence {
        self.head
    }

    /// Version of a live object, or `None` if it does not exist.
    #[must_use]
    pub fn version(&self, object: ObjectRef) -> Option<ObjectVersion> {
        self.records
            .get(&object)
            .filter(|record| !record.deleted)
            .map(|record| record.version)
    }

    /// Version of a deleted object's tombstone, or `None` if it was never
    /// deleted.
    #[must_use]
    pub fn tombstone_version(&self, object: ObjectRef) -> Option<ObjectVersion> {
        self.records
            .get(&object)
            .filter(|record| record.deleted)
            .map(|record| record.version)
    }

    /// Validates every operation in order against a working copy.
    ///
    /// Any failure rejects the whole proposal. This method never mutates the
    /// snapshot and does not read the clock, network, or a random source.
    pub fn preview(&self, proposal: Proposal) -> Result<Preview, Rejection> {
        let mut working = self.project.clone();
        let mut touched: Vec<ObjectRef> = Vec::new();
        let mut touched_set: BTreeSet<ObjectRef> = BTreeSet::new();
        let mut referenced: BTreeSet<ObjectRef> = BTreeSet::new();
        let mut deleted_here: BTreeSet<ObjectRef> = BTreeSet::new();

        for (operation_index, operation) in proposal.operations.iter().enumerate() {
            let ids = UsedIds {
                records: &self.records,
                deleted_here: &deleted_here,
            };
            let effects =
                apply_operation(&mut working, operation, &ids).map_err(|error| Rejection {
                    operation_index,
                    error,
                })?;

            for object in effects.changed {
                if object_state(&working, object).is_none() {
                    deleted_here.insert(object);
                }
                if touched_set.insert(object) {
                    touched.push(object);
                }
            }
            referenced.extend(effects.referenced);
        }

        let changes: Vec<ObjectChange> = touched
            .into_iter()
            .filter_map(|object| {
                let before = object_state(&self.project, object);
                let after = object_state(&working, object);
                (before != after).then(|| ObjectChange {
                    object,
                    base_version: self.version(object),
                    before,
                    after,
                })
            })
            .collect();
        let changed: BTreeSet<ObjectRef> = changes.iter().map(|change| change.object).collect();
        let references = referenced
            .into_iter()
            .filter(|object| !changed.contains(object))
            .collect();

        let mut diagnostics = Vec::new();
        if changes
            .iter()
            .any(|change| change.object.domain() == Domain::Schedule)
        {
            if let Err(ScheduleError::CyclicNetwork { cycles }) = calculate_cpm(&working.schedule) {
                diagnostics.push(Diagnostic::CyclicSchedule { cycles });
            }
        }

        Ok(Preview {
            project_id: self.project.id,
            proposal,
            changes,
            references,
            diagnostics,
        })
    }

    /// Atomically applies a preview if every touched object is still at its
    /// base version, every referenced object still exists, and no live object
    /// still depends on an object it deletes.
    ///
    /// On any failure nothing is written and the head does not move.
    pub fn commit(&mut self, preview: Preview) -> Result<LogEntry, CommitError> {
        if preview.project_id != self.project.id {
            return Err(CommitError::WrongProject {
                preview: preview.project_id,
                snapshot: self.project.id,
            });
        }
        if preview.changes.is_empty() {
            return Err(CommitError::NoChanges);
        }

        let mut conflicts: BTreeSet<ObjectRef> = BTreeSet::new();
        for change in &preview.changes {
            let current = self.records.get(&change.object);
            let unchanged = match (change.base_version, current) {
                (None, None) => true,
                (Some(base), Some(record)) => !record.deleted && record.version == base,
                _ => false,
            };
            if !unchanged {
                conflicts.insert(change.object);
            }
        }
        for object in &preview.references {
            if self.version(*object).is_none() {
                conflicts.insert(*object);
            }
        }
        // A dependent committed after the preview (for example, a new link to
        // a wall this transaction deletes) does not change the deleted
        // object's version, so check deletions against the current state.
        let deleted: BTreeSet<ObjectRef> = preview
            .changes
            .iter()
            .filter(|change| change.after.is_none())
            .map(|change| change.object)
            .collect();
        for object in &deleted {
            conflicts.extend(
                dependents(&self.project, *object)
                    .into_iter()
                    .filter(|dependent| !deleted.contains(dependent)),
            );
        }
        if !conflicts.is_empty() {
            return Err(CommitError::Conflict {
                objects: conflicts.into_iter().collect(),
            });
        }

        for change in &preview.changes {
            apply_change(&mut self.project, change);
            let record = self.records.entry(change.object).or_insert(ObjectRecord {
                version: ObjectVersion(0),
                deleted: false,
            });
            record.version = ObjectVersion(record.version.0 + 1);
            record.deleted = change.after.is_none();
        }
        self.head = LogSequence(self.head.0 + 1);

        let domains: BTreeSet<Domain> = preview
            .changes
            .iter()
            .map(|change| change.object.domain())
            .collect();
        let Proposal {
            id,
            actor,
            rationale,
            operations,
        } = preview.proposal;

        Ok(LogEntry {
            sequence: self.head,
            transaction_id: id,
            actor,
            rationale,
            operations,
            changes: preview.changes,
            domains: domains.into_iter().collect(),
        })
    }
}

/// A proposal that failed validation. Nothing was applied.
#[derive(Debug, Error, PartialEq)]
#[error("operation {operation_index} was rejected: {error}")]
pub struct Rejection {
    /// Zero-based index of the first failing operation.
    pub operation_index: usize,
    pub error: OperationError,
}

#[derive(Debug, Error, PartialEq)]
pub enum OperationError {
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error("{0} does not exist")]
    UnknownObject(ObjectRef),
    #[error("{0} has already been used and cannot be created again")]
    IdAlreadyUsed(ObjectRef),
    #[error("{object} cannot be deleted while {} object(s) depend on it", .dependents.len())]
    HasDependents {
        object: ObjectRef,
        /// Dependents in deterministic order.
        dependents: Vec<ObjectRef>,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommitError {
    #[error("preview belongs to project {preview}, not {snapshot}")]
    WrongProject {
        preview: ProjectId,
        snapshot: ProjectId,
    },
    #[error("transaction has no effective changes")]
    NoChanges,
    #[error("{} object(s) changed or disappeared since the preview", .objects.len())]
    Conflict {
        /// Conflicting objects in deterministic order.
        objects: Vec<ObjectRef>,
    },
}

struct UsedIds<'a> {
    records: &'a BTreeMap<ObjectRef, ObjectRecord>,
    deleted_here: &'a BTreeSet<ObjectRef>,
}

impl UsedIds<'_> {
    /// IDs are never reused: an object that has ever existed, or was deleted
    /// earlier in this transaction, cannot be created again.
    fn ensure_unused(&self, object: ObjectRef) -> Result<(), OperationError> {
        if self.records.contains_key(&object) || self.deleted_here.contains(&object) {
            return Err(OperationError::IdAlreadyUsed(object));
        }
        Ok(())
    }
}

#[derive(Default)]
struct Effects {
    changed: Vec<ObjectRef>,
    referenced: Vec<ObjectRef>,
}

impl Effects {
    fn changed(object: ObjectRef) -> Self {
        Self {
            changed: vec![object],
            referenced: Vec::new(),
        }
    }

    fn referencing(mut self, objects: impl IntoIterator<Item = ObjectRef>) -> Self {
        self.referenced.extend(objects);
        self
    }
}

fn apply_operation(
    project: &mut Project,
    operation: &Operation,
    ids: &UsedIds<'_>,
) -> Result<Effects, OperationError> {
    match operation {
        Operation::RenameProject { name } => {
            project.name.clone_from(name);
            Ok(Effects::changed(ObjectRef::Project(project.id)))
        }

        Operation::CreateLevel(level) => {
            let object = ObjectRef::Level(level.id);
            ids.ensure_unused(object)?;
            project.model.add_level(level.clone())?;
            Ok(Effects::changed(object))
        }
        Operation::RenameLevel { id, name } => {
            let object = ObjectRef::Level(*id);
            let level = project
                .model
                .levels
                .iter_mut()
                .find(|level| level.id == *id)
                .ok_or(OperationError::UnknownObject(object))?;
            level.name.clone_from(name);
            Ok(Effects::changed(object))
        }
        Operation::DeleteLevel { id } => {
            let object = ObjectRef::Level(*id);
            ensure_exists(project, object)?;
            ensure_no_dependents(project, object)?;
            project.model.levels.retain(|level| level.id != *id);
            Ok(Effects::changed(object))
        }
        Operation::CreateWall(wall) => {
            let object = ObjectRef::Wall(wall.id);
            ids.ensure_unused(object)?;
            wall.geometry().validate().map_err(ModelError::from)?;
            project.model.add_wall(wall.clone())?;
            Ok(Effects::changed(object).referencing([ObjectRef::Level(wall.level_id)]))
        }
        Operation::SetWallHeight { id, height } => {
            let wall = find_wall(project, *id)?;
            let mut geometry = wall.geometry();
            geometry.height = *height;
            geometry.validate().map_err(ModelError::from)?;
            wall.height = *height;
            Ok(Effects::changed(ObjectRef::Wall(*id)))
        }
        Operation::SetWallBaseline { id, baseline } => {
            let wall = find_wall(project, *id)?;
            let mut geometry = wall.geometry();
            geometry.baseline = *baseline;
            geometry.validate().map_err(ModelError::from)?;
            wall.baseline = *baseline;
            Ok(Effects::changed(ObjectRef::Wall(*id)))
        }
        Operation::DeleteWall { id } => {
            let object = ObjectRef::Wall(*id);
            ensure_exists(project, object)?;
            ensure_no_dependents(project, object)?;
            project.model.walls.retain(|wall| wall.id != *id);
            Ok(Effects::changed(object))
        }

        Operation::CreateActivity(activity) => {
            let object = ObjectRef::Activity(activity.id);
            ids.ensure_unused(object)?;
            project.schedule.add_activity(activity.clone())?;
            Ok(Effects::changed(object))
        }
        Operation::SetActivityDuration { id, duration_hours } => {
            let object = ObjectRef::Activity(*id);
            let activity = project
                .schedule
                .activities
                .iter_mut()
                .find(|activity| activity.id == *id)
                .ok_or(OperationError::UnknownObject(object))?;
            activity.duration_hours = *duration_hours;
            Ok(Effects::changed(object))
        }
        Operation::DeleteActivity { id } => {
            let object = ObjectRef::Activity(*id);
            ensure_exists(project, object)?;
            ensure_no_dependents(project, object)?;
            project
                .schedule
                .activities
                .retain(|activity| activity.id != *id);
            Ok(Effects::changed(object))
        }
        Operation::CreateRelationship(relationship) => {
            let object = ObjectRef::Relationship(relationship.id);
            ids.ensure_unused(object)?;
            project.schedule.add_relationship(relationship.clone())?;
            Ok(Effects::changed(object).referencing([
                ObjectRef::Activity(relationship.predecessor_id),
                ObjectRef::Activity(relationship.successor_id),
            ]))
        }
        Operation::DeleteRelationship { id } => {
            let object = ObjectRef::Relationship(*id);
            ensure_exists(project, object)?;
            project
                .schedule
                .relationships
                .retain(|relationship| relationship.id != *id);
            Ok(Effects::changed(object))
        }

        Operation::LinkElementActivity(link) => {
            let object = ObjectRef::Link(link.id);
            ids.ensure_unused(object)?;
            project.link_element_activity(link.clone())?;
            Ok(Effects::changed(object).referencing([
                ObjectRef::Wall(link.element_id),
                ObjectRef::Activity(link.activity_id),
            ]))
        }
        Operation::UnlinkElementActivity { id } => {
            let object = ObjectRef::Link(*id);
            ensure_exists(project, object)?;
            project.element_activity_links.retain(|link| link.id != *id);
            Ok(Effects::changed(object))
        }
    }
}

fn find_wall(project: &mut Project, id: ElementId) -> Result<&mut Wall, OperationError> {
    project
        .model
        .walls
        .iter_mut()
        .find(|wall| wall.id == id)
        .ok_or(OperationError::UnknownObject(ObjectRef::Wall(id)))
}

fn ensure_exists(project: &Project, object: ObjectRef) -> Result<(), OperationError> {
    object_state(project, object)
        .map(|_| ())
        .ok_or(OperationError::UnknownObject(object))
}

/// No implicit cascades: deletion is rejected while anything depends on the
/// object, and dependents must be removed explicitly first.
fn ensure_no_dependents(project: &Project, object: ObjectRef) -> Result<(), OperationError> {
    let dependents = dependents(project, object);
    if dependents.is_empty() {
        Ok(())
    } else {
        Err(OperationError::HasDependents {
            object,
            dependents: dependents.into_iter().collect(),
        })
    }
}

/// Live objects that reference `object`, in deterministic order.
///
/// This is the single statement of the reference graph: walls reference
/// levels; relationships reference activities; links reference walls and
/// activities.
fn dependents(project: &Project, object: ObjectRef) -> BTreeSet<ObjectRef> {
    match object {
        ObjectRef::Project(_) | ObjectRef::Relationship(_) | ObjectRef::Link(_) => BTreeSet::new(),
        ObjectRef::Level(id) => project
            .model
            .walls
            .iter()
            .filter(|wall| wall.level_id == id)
            .map(|wall| ObjectRef::Wall(wall.id))
            .collect(),
        ObjectRef::Wall(id) => project
            .element_activity_links
            .iter()
            .filter(|link| link.element_id == id)
            .map(|link| ObjectRef::Link(link.id))
            .collect(),
        ObjectRef::Activity(id) => project
            .schedule
            .relationships
            .iter()
            .filter(|relationship| {
                relationship.predecessor_id == id || relationship.successor_id == id
            })
            .map(|relationship| ObjectRef::Relationship(relationship.id))
            .chain(
                project
                    .element_activity_links
                    .iter()
                    .filter(|link| link.activity_id == id)
                    .map(|link| ObjectRef::Link(link.id)),
            )
            .collect(),
    }
}

fn object_state(project: &Project, object: ObjectRef) -> Option<ObjectState> {
    match object {
        ObjectRef::Project(id) => (project.id == id).then(|| ObjectState::Project {
            name: project.name.clone(),
        }),
        ObjectRef::Level(id) => project
            .model
            .levels
            .iter()
            .find(|level| level.id == id)
            .cloned()
            .map(ObjectState::Level),
        ObjectRef::Wall(id) => project
            .model
            .walls
            .iter()
            .find(|wall| wall.id == id)
            .cloned()
            .map(ObjectState::Wall),
        ObjectRef::Activity(id) => project
            .schedule
            .activities
            .iter()
            .find(|activity| activity.id == id)
            .cloned()
            .map(ObjectState::Activity),
        ObjectRef::Relationship(id) => project
            .schedule
            .relationships
            .iter()
            .find(|relationship| relationship.id == id)
            .cloned()
            .map(ObjectState::Relationship),
        ObjectRef::Link(id) => project
            .element_activity_links
            .iter()
            .find(|link| link.id == id)
            .cloned()
            .map(ObjectState::Link),
    }
}

fn all_objects(project: &Project) -> Vec<ObjectRef> {
    let mut objects = vec![ObjectRef::Project(project.id)];
    objects.extend(
        project
            .model
            .levels
            .iter()
            .map(|level| ObjectRef::Level(level.id)),
    );
    objects.extend(
        project
            .model
            .walls
            .iter()
            .map(|wall| ObjectRef::Wall(wall.id)),
    );
    objects.extend(
        project
            .schedule
            .activities
            .iter()
            .map(|activity| ObjectRef::Activity(activity.id)),
    );
    objects.extend(
        project
            .schedule
            .relationships
            .iter()
            .map(|relationship| ObjectRef::Relationship(relationship.id)),
    );
    objects.extend(
        project
            .element_activity_links
            .iter()
            .map(|link| ObjectRef::Link(link.id)),
    );
    objects
}

/// Writes one validated change into a project: upsert for `after`, removal
/// when `after` is `None`.
fn apply_change(project: &mut Project, change: &ObjectChange) {
    match &change.after {
        Some(ObjectState::Project { name }) => project.name.clone_from(name),
        Some(ObjectState::Level(level)) => {
            upsert(&mut project.model.levels, level.clone(), |item| {
                item.id == level.id
            });
        }
        Some(ObjectState::Wall(wall)) => {
            upsert(&mut project.model.walls, wall.clone(), |item| {
                item.id == wall.id
            });
        }
        Some(ObjectState::Activity(activity)) => {
            upsert(&mut project.schedule.activities, activity.clone(), |item| {
                item.id == activity.id
            });
        }
        Some(ObjectState::Relationship(relationship)) => {
            upsert(
                &mut project.schedule.relationships,
                relationship.clone(),
                |item| item.id == relationship.id,
            );
        }
        Some(ObjectState::Link(link)) => {
            upsert(&mut project.element_activity_links, link.clone(), |item| {
                item.id == link.id
            });
        }
        None => match change.object {
            ObjectRef::Project(_) => {
                unreachable!("operations cannot delete the project itself")
            }
            ObjectRef::Level(id) => project.model.levels.retain(|item| item.id != id),
            ObjectRef::Wall(id) => project.model.walls.retain(|item| item.id != id),
            ObjectRef::Activity(id) => project.schedule.activities.retain(|item| item.id != id),
            ObjectRef::Relationship(id) => {
                project.schedule.relationships.retain(|item| item.id != id);
            }
            ObjectRef::Link(id) => project.element_activity_links.retain(|item| item.id != id),
        },
    }
}

fn upsert<T>(items: &mut Vec<T>, item: T, same: impl Fn(&T) -> bool) {
    match items.iter_mut().find(|existing| same(existing)) {
        Some(existing) => *existing = item,
        None => items.push(item),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementActivityRole;
    use rustit_geometry::Point3;
    use rustit_schedule::RelationshipType;

    fn carl() -> Actor {
        Actor::new(ActorKind::Human, "carl")
    }

    fn segment(length: f64) -> Segment3 {
        Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(length, 0.0, 0.0))
    }

    fn base() -> (ProjectSnapshot, Level) {
        let mut project = Project::new("Safe Room unit test");
        let level = Level::new("Dungeon Floor 1", 0.0);
        project.model.add_level(level.clone()).expect("level");
        (ProjectSnapshot::genesis(project), level)
    }

    fn commit(snapshot: &mut ProjectSnapshot, operations: Vec<Operation>) -> LogEntry {
        let preview = snapshot
            .preview(Proposal::new(carl(), "unit test", operations))
            .expect("valid proposal");
        snapshot.commit(preview).expect("commit")
    }

    /// A wall, its activity, and the link between them commit together as one
    /// cross-domain transaction. Objects created in the same transaction are
    /// changes, not references.
    #[test]
    fn creates_model_schedule_and_link_atomically() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        let activity = Activity::new("Carl frames wall", 8);
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);

        let preview = snapshot
            .preview(Proposal::new(
                carl(),
                "Frame the first wall",
                vec![
                    Operation::CreateWall(wall.clone()),
                    Operation::CreateActivity(activity.clone()),
                    Operation::LinkElementActivity(link.clone()),
                ],
            ))
            .expect("valid proposal");

        assert_eq!(
            preview
                .changes()
                .iter()
                .map(|change| change.object)
                .collect::<Vec<_>>(),
            vec![
                ObjectRef::Wall(wall.id),
                ObjectRef::Activity(activity.id),
                ObjectRef::Link(link.id),
            ]
        );
        assert!(
            preview
                .changes()
                .iter()
                .all(|change| change.base_version.is_none() && change.before.is_none())
        );
        assert_eq!(preview.references(), &[ObjectRef::Level(level.id)]);
        assert_eq!(snapshot.head(), LogSequence(0));

        let entry = snapshot.commit(preview).expect("commit");

        assert_eq!(entry.sequence, LogSequence(1));
        assert_eq!(
            entry.domains,
            vec![Domain::Model, Domain::Schedule, Domain::FourD]
        );
        assert_eq!(
            snapshot.version(ObjectRef::Link(link.id)),
            Some(ObjectVersion(1))
        );
        assert_eq!(snapshot.project().element_activity_links, vec![link]);
    }

    #[test]
    fn a_no_op_edit_produces_no_change_and_cannot_commit() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        commit(&mut snapshot, vec![Operation::CreateWall(wall.clone())]);

        let preview = snapshot
            .preview(Proposal::new(
                carl(),
                "Set the same height",
                vec![Operation::SetWallHeight {
                    id: wall.id,
                    height: 3.0,
                }],
            ))
            .expect("valid proposal");

        assert!(preview.changes().is_empty());
        assert_eq!(snapshot.commit(preview), Err(CommitError::NoChanges));
        assert_eq!(
            snapshot.version(ObjectRef::Wall(wall.id)),
            Some(ObjectVersion(1))
        );
    }

    #[test]
    fn deleted_ids_are_never_reused() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        commit(&mut snapshot, vec![Operation::CreateWall(wall.clone())]);
        commit(&mut snapshot, vec![Operation::DeleteWall { id: wall.id }]);

        assert_eq!(snapshot.version(ObjectRef::Wall(wall.id)), None);
        assert_eq!(
            snapshot.tombstone_version(ObjectRef::Wall(wall.id)),
            Some(ObjectVersion(2))
        );
        assert_eq!(
            snapshot.preview(Proposal::new(
                carl(),
                "Recreate",
                vec![Operation::CreateWall(wall.clone())]
            )),
            Err(Rejection {
                operation_index: 0,
                error: OperationError::IdAlreadyUsed(ObjectRef::Wall(wall.id)),
            })
        );
    }

    #[test]
    fn concurrent_creation_of_the_same_id_conflicts() {
        let (mut snapshot, _) = base();
        let activity = Activity::new("Mordecai briefs crew", 1);
        let proposal = || {
            Proposal::new(
                carl(),
                "Create",
                vec![Operation::CreateActivity(activity.clone())],
            )
        };
        let first = snapshot.preview(proposal()).expect("valid");
        let second = snapshot.preview(proposal()).expect("valid");

        snapshot.commit(first).expect("first commit");

        assert_eq!(
            snapshot.commit(second),
            Err(CommitError::Conflict {
                objects: vec![ObjectRef::Activity(activity.id)]
            })
        );
    }

    /// RFC 0004 fixture 5 in miniature: a link fails to commit if its
    /// referenced activity was deleted after the preview.
    #[test]
    fn a_reference_deleted_after_preview_conflicts() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        let activity = Activity::new("Carl frames wall", 8);
        commit(
            &mut snapshot,
            vec![
                Operation::CreateWall(wall.clone()),
                Operation::CreateActivity(activity.clone()),
            ],
        );
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);
        let link_preview = snapshot
            .preview(Proposal::new(
                carl(),
                "Link",
                vec![Operation::LinkElementActivity(link)],
            ))
            .expect("valid");

        commit(
            &mut snapshot,
            vec![Operation::DeleteActivity { id: activity.id }],
        );

        assert_eq!(
            snapshot.commit(link_preview),
            Err(CommitError::Conflict {
                objects: vec![ObjectRef::Activity(activity.id)]
            })
        );
        assert_eq!(snapshot.head(), LogSequence(2));
    }

    /// The mirror of reference survival: a link committed after a delete was
    /// previewed must block the delete, even though the wall's own version did
    /// not change. Otherwise the link would dangle.
    #[test]
    fn a_dependent_committed_after_a_delete_preview_conflicts() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        let activity = Activity::new("Carl frames wall", 8);
        commit(
            &mut snapshot,
            vec![
                Operation::CreateWall(wall.clone()),
                Operation::CreateActivity(activity.clone()),
            ],
        );
        let delete_preview = snapshot
            .preview(Proposal::new(
                carl(),
                "Remove wall",
                vec![Operation::DeleteWall { id: wall.id }],
            ))
            .expect("valid while unlinked");
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);
        commit(
            &mut snapshot,
            vec![Operation::LinkElementActivity(link.clone())],
        );

        assert_eq!(
            snapshot.commit(delete_preview),
            Err(CommitError::Conflict {
                objects: vec![ObjectRef::Link(link.id)]
            })
        );
        assert_eq!(
            snapshot.version(ObjectRef::Wall(wall.id)),
            Some(ObjectVersion(1))
        );
    }

    #[test]
    fn a_cycle_is_a_diagnostic_not_a_rejection() {
        let (mut snapshot, _) = base();
        let first = Activity::new("Form", 8);
        let second = Activity::new("Pour", 4);
        let preview = snapshot
            .preview(Proposal::new(
                carl(),
                "Stage a loop while re-sequencing",
                vec![
                    Operation::CreateActivity(first.clone()),
                    Operation::CreateActivity(second.clone()),
                    Operation::CreateRelationship(ActivityRelationship::new(
                        first.id,
                        second.id,
                        RelationshipType::FinishStart,
                        0,
                    )),
                    Operation::CreateRelationship(ActivityRelationship::new(
                        second.id,
                        first.id,
                        RelationshipType::FinishStart,
                        0,
                    )),
                ],
            ))
            .expect("cycles do not reject");
        let mut members = vec![first.id, second.id];
        members.sort_by_key(|id| id.as_uuid());

        assert_eq!(
            preview.diagnostics(),
            &[Diagnostic::CyclicSchedule {
                cycles: vec![members]
            }]
        );
        snapshot
            .commit(preview)
            .expect("a diagnostic does not block commit");
    }

    #[test]
    fn deleting_a_level_with_walls_names_its_dependents() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        commit(&mut snapshot, vec![Operation::CreateWall(wall.clone())]);

        assert_eq!(
            snapshot.preview(Proposal::new(
                carl(),
                "Delete level",
                vec![Operation::DeleteLevel { id: level.id }]
            )),
            Err(Rejection {
                operation_index: 0,
                error: OperationError::HasDependents {
                    object: ObjectRef::Level(level.id),
                    dependents: vec![ObjectRef::Wall(wall.id)],
                },
            })
        );
    }

    #[test]
    fn log_entries_round_trip_through_json() {
        let (mut snapshot, level) = base();
        let wall = Wall::new(level.id, segment(5.0), 0.2, 3.0).expect("wall");
        let entry = commit(&mut snapshot, vec![Operation::CreateWall(wall)]);

        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: LogEntry = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored, entry);
    }
}
