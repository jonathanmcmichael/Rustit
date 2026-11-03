# RFC 0004: Transactions, Object Versions, and the Change Log

- Status: Accepted
- Authors: Jonathan McMichael (drafted with AI assistance)
- Created: 2026-11-03
- Accepted: 2026-11-03 by the bootstrap maintainer, with the reviewer decisions in PR #46 (no implicit cascades, reference survival, cycles as diagnostics, one log per project, no CRDTs) accepted as written
- Related: RFC 0001 (unresolved question: "smallest deterministic transaction and dependency model"); RFC 0003 (units and tolerances, #12); calendars RFC (#18)

## Summary

Every change to a Rustit project becomes a typed **transaction**: an ordered list of intent-level operations on stable IDs, proposed by an identified actor with a stated rationale. A transaction is validated in full against a working copy and produces a reviewable **change set** of object-level before/after states. Committing it is atomic. Each canonical object carries its own **version**, and commits are checked object by object, so two people editing different objects never conflict. A modeler and a scheduler can edit the same project at the same time. Each committed transaction is appended to a per-project, totally ordered **change log**. Replaying the log reproduces the project, and the log is the history, audit trail, and change feed for persistence, sync, and automation.

This RFC replaces whole-project save-and-version as the concurrency boundary in RFC 0001 and changes the `ProjectRepository` port. It does not choose a PostgreSQL schema. That is a follow-up RFC built on these semantics.

## Motivation

Today `ProjectRepository::save(project, expected_version)` writes the whole `Project` under one project-wide version. That shape blocks the platform Rustit intends to be:

- **Separate scheduling is impossible in practice.** A scheduler changing one activity duration and a modeler changing one wall height conflict, because both bump the same project version. BIM and scheduling are peers in the domain model but not in persistence.
- **Persistence cost grows with project size, not change size.** Saving one edited wall rewrites every object. "What changed since yesterday?" requires diffing whole projects.
- **There is no history or audit.** VISION.md requires that changes be validated, previewed, diffed, approved, committed, audited, and reversed, and that people, integrations, and AI share one mutation path. There is currently no mutation path at all: callers edit public fields directly.
- **Sync and adapters have nothing to plan against.** Plan-then-apply synchronization needs a reviewable change set and a feed of changes since the last sync.

The smallest model that fixes all four is the same model: typed transactions, per-object versions, and an append-only log.

## Decision

### Terms

```text
Operation    an intent on stable IDs: "set wall W height to 3.2 m"
Proposal     ordered operations + actor + rationale, not yet validated
Preview      a validated proposal: its change set and diagnostics, nothing written
ChangeSet    object-level effects: for each touched object, base version, before, after
Commit       atomically apply a previewed change set if every base version still matches
LogEntry     the committed transaction, appended at the next log sequence number
```

### 1. Operations are typed intents on stable IDs

Operations form a closed, serializable enum grouped by domain. The initial set covers exactly the objects that exist today:

```rust
pub enum Operation {
    // Project
    RenameProject { name: String },

    // BIM model
    CreateLevel(Level),
    RenameLevel { id: LevelId, name: String },
    DeleteLevel { id: LevelId },
    CreateWall(Wall),
    SetWallHeight { id: ElementId, height: f64 },
    SetWallBaseline { id: ElementId, baseline: Segment3 },
    DeleteWall { id: ElementId },

    // Schedule
    CreateActivity(Activity),
    SetActivityDuration { id: ActivityId, duration_hours: u32 },
    DeleteActivity { id: ActivityId },
    CreateRelationship(ActivityRelationship),
    DeleteRelationship { id: ActivityRelationshipId },

    // 4D
    LinkElementActivity(ElementActivityLink),
    UnlinkElementActivity { id: ElementActivityLinkId },
}
```

Rules:

- **Callers generate creation IDs.** A `CreateWall` carries the wall's UUID, so a later operation in the same transaction can reference it. Offline clients, adapters, and AI can also build multi-step proposals without a server round trip.
- **IDs are never reused.** Deleting an object leaves a tombstone version, and creating an object whose ID has existed is rejected.
- **No implicit cascades.** Deleting a wall that has 4D links, or an activity that has relationships or links, is rejected unless the same transaction explicitly removes those dependents first. Explicit operations keep every effect visible in the change set and the audit trail.
- **New operations are additive.** Adding an operation for a new object type or property is an ordinary compatible change. Changing the meaning of an existing operation is a boundary change.

Intent-level operations, rather than generic "put object" writes, are what make audit trails readable ("Mongo changed wall height from 3.0 to 3.2 m") and let validation reuse the domain constructors that already exist (`Wall::new`, `Schedule::add_relationship`, `Project::link_element_activity`).

### 2. Validation is total and atomic

`preview(&project, proposal) -> Result<Preview, Rejection>` is a pure function in `rustit-core`:

1. Apply the operations, in order, to a working copy of the project.
2. Each operation must satisfy the same invariants the domain constructors enforce today. Examples: positive finite wall dimensions, existing levels, known activities on both ends of a relationship, and existing elements and activities for a 4D link.
3. After the last operation, check project-level invariants that individual operations cannot see, such as no dangling references. CPM acyclicity is reported as a **diagnostic**, not a rejection, so a scheduler can stage an intermediate network. That policy is revisited when CPM diagnostics mature (#16).
4. Any failure rejects the whole proposal. A `Rejection` names the failing operation index and the domain error, and nothing is applied.

Validation never reads the clock, the network, or a random source, so the same project and proposal always produce the same preview.

### 3. Change sets describe effects, not intents

A preview's change set lists every touched object exactly once, in first-touch order:

```rust
pub struct ObjectChange {
    pub object: ObjectRef,                  // typed: Wall(ElementId), Activity(ActivityId), ...
    pub base_version: Option<ObjectVersion>, // None: the object must not exist yet
    pub before: Option<ObjectState>,         // None: created
    pub after: Option<ObjectState>,          // None: deleted
}
```

Change sets are the unit of review (diff display), commit checking, persistence, sync planning, and undo. Operations are retained beside them as the human- and agent-readable explanation.

### 4. Every canonical object has its own version

Each `Level`, `Wall`, `Activity`, `ActivityRelationship`, `ElementActivityLink`, and the project's own attributes carry an `ObjectVersion(u64)`. It starts at 1 on creation and increases by one on every committed change, including deletion.

**Commit rule.** A commit succeeds only if, for every `ObjectChange`, the object's current version equals `base_version` (or the object is still absent when `base_version` is `None`). In addition, every object the transaction **references but does not change** must still exist. Linking wall W to activity A requires W and A to exist at commit, but not to be unchanged.

**On conflict** the commit returns `CommitError::Conflict { objects }`, listing every conflicting `ObjectRef` in a deterministic order, and writes nothing. Because operations are intents, the caller recovers by previewing the same proposal against the new state. If no touched object conflicted, there was no conflict to recover from.

**Consequences for separated scheduling.** Model edits and schedule edits touch disjoint objects, so they never conflict with each other. They meet only where they genuinely interact: a 4D link, or deleting an object that a link references. The project-wide version disappears as a concurrency boundary. A stale view of the project is not a conflict; a stale view of a touched object is.

### 5. The change log is append-only and totally ordered

Each commit appends one entry at the next per-project `LogSequence(u64)`:

```rust
pub struct LogEntry {
    pub sequence: LogSequence,
    pub transaction_id: TransactionId,
    pub actor: Actor,               // kind (Human, Integration, Agent) + opaque actor ID
    pub rationale: String,
    pub operations: Vec<Operation>,
    pub changes: Vec<ObjectChange>,
    pub domains: DomainSet,         // Model, Schedule, FourD, Project: derived from changes
    pub recorded_at: String,        // informational metadata; never used for ordering
}
```

Invariants:

- **Replay determinism.** Applying the `changes` of entries 1..=n, in order, to an empty project yields exactly the state at sequence n. This is the core truth-lab invariant for the whole persistence layer.
- **The log is never rewritten.** Undo is a new transaction built from the `before` states of an earlier change set. It goes through the same commit rule, so it conflicts if someone has since changed those objects.
- **One log per project, filtered by domain.** Cross-domain transactions (create a wall, create its activity, link them) stay atomic. Consumers such as a P6 adapter can follow only `Schedule` entries, and a renderer can follow only `Model` entries.
- **The head sequence replaces the project version** as a cheap "has anything changed?" check. It is not used for conflict detection.
- **Actors are recorded, not authorized.** Authentication, permissions, and approval workflows are out of scope. They attach to proposals later without changing this structure.

### 6. One mutation path for every client

- **Desktop and CLI** build proposals from user commands.
- **Adapters** import by producing a proposal: canonical objects and changes to them, plus the external-identity mappings they already return. Their import plan is a preview. This extends plan-then-apply to inbound sync.
- **AI agents** produce proposals with a rationale. A human, or a later approval policy, commits the preview. Agents receive no other write path.

Direct mutation of `Project` fields remains possible in-process for tests and construction, but no repository or adapter accepts a project except through a commit.

### 7. Crate placement and port changes

- Operation, proposal, preview, change set, version, and log types, plus `preview` and an in-memory `apply`, live in `rustit-core`. They are pure, serializable, and free of database dependencies.
- `rustit-postgres::ProjectRepository` changes from whole-project save to:

```rust
pub trait ProjectRepository: Send + Sync {
    fn load_snapshot(&self, id: ProjectId) -> RepositoryFuture<'_, Option<ProjectSnapshot>>;
    fn commit(&self, id: ProjectId, preview: Preview) -> RepositoryFuture<'_, CommitReceipt>;
    fn changes_since(&self, id: ProjectId, after: LogSequence)
        -> RepositoryFuture<'_, Vec<LogEntry>>;
}
```

  `ProjectSnapshot` is the project plus every object's version and the head sequence. `CommitReceipt` returns the new sequence and versions. `RepositoryError::VersionConflict` becomes the object-level `Conflict` above.
- An in-memory repository implementing this port is the reference implementation and the truth-lab harness (it supersedes the whole-project shape assumed by #21).

### 8. Implications for the PostgreSQL schema (decided in a follow-up RFC)

This RFC constrains, but does not define, the schema:

- Objects are stored one row per object with `(project_id, object_id, kind, version, deleted)` plus their data. Writes are proportional to the change set, not the project.
- A commit is one database transaction that conditionally updates each touched row on its base version, verifies referenced rows exist, inserts the log entry, and advances the head sequence. Any failed condition rolls back everything.
- Model, schedule, and 4D-link objects live in separate tables, so each domain can be indexed, queried, and later partitioned on its own terms (PostGIS on model geometry, for example) while sharing one log.
- The log doubles as a change feed for caches, subscriptions, and adapters.

## Acceptance fixtures for the first implementation

Each becomes a truth-lab case with fixed UUIDs:

1. **Separated edits.** From the same base, a modeler sets wall W's height and a scheduler sets activity A's duration. Both commit, and the log has two entries.
2. **Overlapping edits.** Two proposals set wall W's height from the same base. The second fails with `Conflict { objects: [Wall(W)] }` and nothing is written.
3. **Atomic rejection.** A proposal whose third operation makes a wall invalid leaves the project, versions, and log unchanged, and names operation index 2.
4. **No cascades.** Deleting a linked wall is rejected. The same deletion preceded by `UnlinkElementActivity` succeeds.
5. **Reference survival.** A link commit fails if the referenced activity was deleted after the preview.
6. **Replay.** Replaying the fixture's log onto an empty project equals the stored snapshot, including versions.
7. **Undo.** Undoing a height change restores the prior value at a new sequence. Undoing it again after another user's edit conflicts.

## Consequences

- Modelers, schedulers, integrations, and agents can work concurrently. Conflicts appear only where their work truly overlaps.
- Writes, sync, and change notification cost O(change), not O(project).
- History, audit, diff review, and undo come from one structure instead of four.
- Every new editable property needs an operation, validation, and a fixture. That is more ceremony than public fields, and it is deliberate.
- Serialized operations become a long-lived format: renaming or reinterpreting one later is a migration.
- The current `ProjectRepository` and the `ProjectVersion` type are replaced. No external implementations exist yet, so the break is cheap now and expensive later.

## Rejected alternatives

### Keep whole-project optimistic versioning

It is simple, but it makes every pair of concurrent edits a conflict, and it makes writes and diffs proportional to project size. It defeats separate scheduling at the storage layer.

### Separate logs per domain (model log, schedule log)

This sounds like stronger separation, but it loses atomic cross-domain transactions. Creating a wall, its activity, and the 4D link between them must commit or fail together. Per-object versions already give domains independent concurrency, and domain tags give them independent feeds.

### Generic "put object" writes without intent operations

These are easier to implement, but audit trails become JSON diffs with no meaning, validation must rediscover intent, and agent proposals become unreviewable blobs.

### CRDTs or automatic merging (e.g. Automerge)

These are attractive for offline editing, but merges are accepted without domain validation and can produce states no single actor proposed, such as a wall with an invalid baseline or a cyclic schedule. Rustit prefers explicit conflicts on validated transactions. Offline merge policy remains open and may layer on top of this model.

### Database triggers or temporal tables as the history mechanism

These tie history to PostgreSQL and leave local, in-memory, and offline use without it. The log is a domain concept, and PostgreSQL stores it.

### Pure event sourcing without snapshots

Replaying the entire log for every load scales poorly. Rustit stores current state and the log together. Replay is the verification invariant, not the load path.

## Unresolved questions

- **Branches and merging.** How does a "proposed schedule scenario" diverge from and rejoin the main log?
- **Offline.** Can an offline client accumulate proposals and commit them on reconnect with per-object conflicts, or does it need a merge policy?
- **Approval and permissions.** Who may commit an agent's or integration's preview, and how is the approval recorded?
- **Operation schema evolution.** How are old log entries read after an operation's payload changes?
- **Large imports.** Is a 100,000-object import one transaction, or a recorded batch of transactions? What are the memory and latency limits?
- **Retention and compaction.** Can old log entries be archived while preserving audit requirements?
- **Derived data.** CPM and 4D results are computed, not stored. Should cached derived results be keyed by log sequence?

These should be decided with fixtures as their slices begin, and should not block the first implementation.
