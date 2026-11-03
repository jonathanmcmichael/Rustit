# RFC 0005: PostgreSQL and PostGIS Persistence

- Status: Proposed
- Authors: Jonathan McMichael (drafted with AI assistance)
- Created: 2026-11-03
- Related: RFC 0001 (persistence), RFC 0004 (transactions, object versions, change log; §7–8 constrain this RFC), RFC 0003 (units, tolerances, coordinate systems, #12)

## Summary

Rustit's first shared persistence backend stores the project as **one row per canonical object** in per-kind PostgreSQL tables, beside an **append-only change-log table**, inside a configurable schema.

- Each row has typed columns for identity, version, tombstone, references, and the fields worth indexing. It also holds the complete canonical object as `jsonb`.
- Wall baselines are also projected into a PostGIS column for spatial queries.
- A commit is one database transaction. It locks the project's log head, conditionally writes each changed row on its base version, verifies references, and appends the log entry. Any failed condition rolls everything back.

The repository port moves from `rustit-postgres` into `rustit-core` so that the in-memory, PostgreSQL, and future local backends all implement one contract and pass one shared test suite.

The initial driver is `sqlx` using runtime-checked queries. The local/offline store is deferred, but driver selection keeps SQLite open.

This RFC also sets benchmark targets, so that "fast" is an acceptance criterion rather than an adjective.

## Motivation

RFC 0004 replaced whole-project saves with transactions and object versions, and stated what a schema must provide: writes proportional to the change set, conditional per-object updates, the log insert in the same database transaction, and separate domain tables. `rustit-postgres` still defines only a whole-project port, with no driver, schema, or migrations.

A VDC platform's database is the product, not a backing file. It must:

- let modelers, schedulers, integrations, and agents commit concurrently (RFC 0004);
- answer project questions without loading the whole project: what changed since the last sync, the history of wall W, walls inside this zone;
- feed 4D, dashboards, and adapters from the change log; and
- stay open: plain PostgreSQL and PostGIS that any SQL tool can read.

## Decision

### 1. The repository port moves into `rustit-core`

```rust
// rustit-core::repository
pub trait ProjectRepository: Send + Sync {
    fn create(&self, project: Project) -> RepositoryFuture<'_, ProjectSnapshot>;
    fn load_snapshot(&self, id: ProjectId) -> RepositoryFuture<'_, Option<ProjectSnapshot>>;
    fn commit(&self, preview: Preview) -> RepositoryFuture<'_, LogEntry>;
    fn changes_since(&self, id: ProjectId, after: LogSequence)
        -> RepositoryFuture<'_, Vec<LogEntry>>;
}
```

- The port stays runtime-agnostic: boxed `Send` futures, no driver types.
- An in-memory implementation lives beside the port. It is the reference implementation and the harness for the RFC 0004 fixtures.
- `rustit-postgres` depends on `rustit-core` and implements the port; nothing in the domain depends on `rustit-postgres`.
- `RepositoryError` gains `Commit(CommitError)`, so callers see the same `Conflict { objects }` from every backend.
- `create` is the genesis of a project's history. It writes the initial objects at version 1 and the head at sequence 0, matching `ProjectSnapshot::genesis`.

This is the RFC 0004 §7 port, with `create` added and with its home made explicit. No crate depends on the current `rustit-postgres` port.

### 2. Driver: `sqlx`, runtime-checked queries

**Decision:** use `sqlx` with the Postgres and `rustls` features, plain `sqlx::query` (runtime-checked, no compile-time macros), and the embedded `sqlx::migrate!` migrator.

Why:

- **No database at build time.** Compile-time-checked macros would require a live database or a committed query cache for every contributor and CI job. Runtime queries keep `cargo xtask verify` database-free. The shared contract suite (§9) provides the checking the macros would have.
- **Migrations ship in the binary.** There is no separate migration tool to install or version.
- **SQLite stays open** for the local/offline store (deferred, §10) without a second driver ecosystem.
- `rustls` avoids a native OpenSSL dependency on Windows and Linux.

What we give up: `tokio-postgres` is lower-level and exposes the binary protocol and `COPY` more directly. `sqlx` exposes `COPY` through raw copy-in and supports batched `UNNEST` inserts, which §6 relies on. If benchmarks show the driver itself is the bottleneck, swapping it is confined to `rustit-postgres`.

The implementation requires a Tokio runtime inside `rustit-postgres` only. The desktop app keeps using the in-memory repository until a persistence UI slice exists.

### 3. Schema layout: typed columns plus canonical `jsonb`

Every object table has the same spine:

```text
project_id      uuid      not null
<object>_id     uuid      not null        -- the canonical Rustit UUID, never a surrogate key
version         bigint    not null        -- RFC 0004 ObjectVersion
deleted         boolean   not null        -- tombstone; IDs are never reused
created_seq     bigint    not null        -- log sequence that created or last restored the object (0 at genesis)
created_ord     integer   not null        -- position within that change set
data            jsonb     not null        -- complete serialized object state
primary key (project_id, <object>_id)
```

- **`data` is canonical.** It is the serde form of the domain struct, and loading deserializes it. A loaded object must equal the committed object exactly; the round-trip fixture in §9 checks this for every type, including `f64` fields.
- **Typed columns are projections** of `data`, written in the same statement: references, fields worth filtering or indexing, and spatial columns. They exist for integrity, indexes, and SQL readers, and are never read back as truth. Adding a projected column is a migration. Adding a domain field that is not projected is not.
- **Load order is deterministic.** Rows load in `(created_seq, created_ord)` order. That reproduces the order the in-memory commit appends objects, so a loaded snapshot equals the in-memory one. An undo that restores a tombstoned object appends it again, so restoration resets `created_seq` and `created_ord` to the restoring commit.

Tables (projections beyond the spine shown):

```text
project              project_id PK, name, ifc_schema, version, head_seq
element              (project_id, element_id) PK, kind            -- identity registry for every element type
level                + name, elevation
wall                 + level_id → level, thickness, height,
                       baseline geometry(LineStringZ, 0)          -- PostGIS projection, GiST-indexed
                       (project_id, wall_id) → element
activity             + name, duration_hours, status
activity_relationship + predecessor_id → activity, successor_id → activity, relationship_type, lag_hours
element_activity_link + element_id → element, activity_id → activity, role
change_log           (project_id, sequence) PK, transaction_id unique, actor_kind, actor_id, rationale,
                       domains text[], operations jsonb, changes jsonb, recorded_at timestamptz
object_change        (project_id, sequence, object_kind, object_id) PK, version   -- per-object history index
```

Notes:

- **Model, schedule, and 4D tables are separate.** Each domain gets its own indexes and its own query plans, and can later be partitioned independently. They share one log, as RFC 0004 requires.
- **Foreign keys protect identity, not liveness.** A foreign key to a tombstoned row is still satisfied. Liveness at commit is enforced by the RFC 0004 commit rule (§5), so the database still never holds a dangling ID.
- **The `element` registry** lets 4D links reference any future element type (slabs, columns, openings) without a foreign key per table.
- **`object_change`** answers "history of wall W" and "who last touched this activity" with an index lookup instead of scanning `changes` JSON.
- **`recorded_at`** is set by the database at commit. It is informational and never used for ordering (RFC 0004 §5).

### 4. PostGIS in local project coordinates

Wall baselines are stored as `geometry(LineStringZ, 0)`, built in SQL from the baseline coordinates with `ST_MakeLine(ST_MakePoint(...), ST_MakePoint(...))`. No Rust geometry crate is needed.

- SRID 0 means local engineering coordinates in metres. Mapping to a real CRS waits for RFC 0003 and stays out of the domain model.
- Spatial columns are projections. Meshes and B-rep solids are generated products and are **not** stored in the database (VISION §7).

### 5. Commit protocol

One database transaction at `READ COMMITTED`:

1. `SELECT head_seq FROM project WHERE project_id = $1 FOR UPDATE`. This serializes commits **per project**, which the total log order requires, while different projects commit in parallel. Commits are short, so this lock does not serialize editing, only the brief write.
2. Check every `ObjectChange` against its row. The row's version must equal `base_version` and the row must not be deleted; for creations, the row must not exist. Check every referenced object for a live row. For every object the change set deletes, check that no live row outside the change set still references it. A dependent committed after the preview does not change the deleted object's version, so the version check alone would miss it. Collect **all** failures before deciding, so the error lists the complete, deterministically ordered conflict set, exactly as the in-memory backend does.
3. If any check failed, roll back and return `Conflict { objects }`.
4. Write the changes:
   - creations as batched inserts;
   - updates as `UPDATE … SET data = $, <projections>, version = version + 1 WHERE … AND version = $base`;
   - deletions as tombstone updates.

   Each conditional write must affect exactly one row. Under the head lock this always holds, and the conditions remain as defense in depth.
5. Insert the `change_log` row and its `object_change` rows, set `head_seq`, and commit.
6. After commit, `NOTIFY rustit_changes` with `project_id` and `sequence` (no object data). Subscribers then call `changes_since`.

### 6. Bulk change sets

Large imports, such as an initial P6 schedule or an IFC model, are single RFC 0004 transactions. Rows are written with `UNNEST`-array statements per table, in batches. If benchmarks require it, they can instead be written with `COPY` into a temporary table followed by a merge. The commit protocol does not change.

### 7. Derived data may be cached, never trusted

CPM timings and 4D states are computed from the snapshot in Rust and are not authored truth. A later slice may add derived cache tables (for example, `cpm_timing` keyed by `(project_id, computed_at_seq, activity_id)`) so that SQL and dashboards can answer "what is built at hour *t*" with a join. Rules for any cache:

- it is keyed by the log sequence it was computed from, and is stale once the head moves past that sequence;
- it can be rebuilt from the snapshot at any time; and
- it is never read by `load_snapshot` or used to validate a commit.

### 8. Configuration and security

- Tables live in `PostgresOptions::schema` (default `rustit`). The database URL is read from `RUSTIT_DATABASE_URL` at runtime and is never committed (existing behavior).
- Migrations are embedded, forward-only, and run explicitly by an operator command or application startup flag, not implicitly on every connection.
- The application role needs only DML on the Rustit schema; migrations run under a separate owner role. Row-level security and permissions follow the RFC 0004 approval work.

### 9. Verification

- **One contract suite for every backend.** RFC 0004 fixtures 1–7, plus a snapshot round trip covering every object type, run unchanged against the in-memory and PostgreSQL repositories. Matching results are the evidence that the backends agree.
- **`cargo xtask verify` stays database-free.** PostgreSQL integration tests skip with a clear message when `RUSTIT_DATABASE_URL` is unset.
- **CI adds a job** with a PostGIS service container that sets the variable and runs the integration tests.
- **Benchmarks** run with a deterministic synthetic project generator. The generator is checked in; no customer models are used. Targets below are initial and are revised only with recorded evidence.

| Operation (synthetic mid-size project: 10,000 walls, 5,000 activities, 10,000 relationships, 10,000 links) | Initial target |
| --- | --- |
| Commit a single-object edit | p95 under 10 ms |
| Commit a 1,000-object change set | under 250 ms |
| Create the full project (~35,000 objects) in one transaction | under 5 s |
| `load_snapshot` of the full project | under 1 s |
| `changes_since` returning 100 entries | under 20 ms |

These targets assume PostgreSQL on the same machine with local SSD storage. They are acceptance gates for the implementation packets, not claims about current behavior.

### 10. Deferred: local and offline storage

Rustit must eventually work without a server. This RFC does not choose that store. It does fix two constraints:

- the serialized `LogEntry` stream is the portable interchange between backends; and
- any local store implements the same port and passes the same contract suite.

SQLite through `sqlx` is the leading candidate.

## Implementation packets

1. **Port move and in-memory reference implementation.** Move `ProjectRepository` into `rustit-core`, add the in-memory repository and contract suite, and remove the old whole-project port. This completes RFC 0004 packet 3.
2. **Schema and migrations.** Add `sqlx` to `rustit-postgres`, write the first migration, add the PostGIS CI job, and add skipped-by-default integration tests.
3. **Repository implementation.** Implement `create`, `load_snapshot`, `commit`, and `changes_since` until the shared contract suite passes against PostgreSQL.
4. **Benchmarks.** Add the synthetic generator and benchmark harness, and record results against the targets in the PR.
5. **Optional:** the `NOTIFY` feed consumer, and a derived CPM cache with an SQL 4D view.

## Consequences

- Writes, history lookups, and sync feeds cost O(change), and any SQL client can read the project.
- Spatial queries over the model are available from day one, without making PostGIS the source of truth.
- Every projected field is two representations of one value, which a test must keep consistent.
- `sqlx` adds a substantial dependency tree to `rustit-postgres` only. The security audit workflow covers it, and no domain crate is affected.
- The per-project head lock caps commit throughput per project at roughly one commit per database round trip. That is far above interactive editing rates, but it is a known ceiling for very high-volume automated writers.

## Rejected alternatives

### Store the whole project as one `jsonb` document

This is what the current port implies. Every save rewrites the project, every concurrent pair of edits conflicts, and nothing is queryable. RFC 0004 already rejected it.

### One generic `objects` table with `kind` and `jsonb`

It is flexible, but it loses foreign keys, per-domain indexes, and the separation of model and schedule storage. Query plans suffer as one table mixes every kind of object.

### Fully normalized columns for every field

Every domain field becomes a migration, and nested values such as classifications become child tables before any query needs them. Projections give the integrity and indexes where they matter; `jsonb` keeps exact round trips everywhere else.

### `tokio-postgres` (with `deadpool` and a separate migration tool)

It is a strong, lower-level option. It was rejected for now because migrations would need another tool and SQLite would need another driver ecosystem. It remains the fallback if benchmarks implicate the driver.

### Diesel or SeaORM

An ORM model would duplicate the domain types that Rustit already owns and would pull persistence concerns into them. Rustit maps its own types at the adapter boundary.

### Database triggers or temporal tables for history

RFC 0004 rejected these. History is the domain change log, which every backend must produce identically.

### Storing meshes or solids in the database

They are generated products. Storing them invites treating them as truth, and they can be regenerated from semantic objects.

## Unresolved questions

- **Coordinate reference systems:** when does SRID 0 become a real CRS, and is that per project or per site (RFC 0003)?
- **Very large logs:** should `change_log` be partitioned (by project or by sequence range), and how is it retained or archived (RFC 0004)?
- **Element geometry beyond walls:** footprints, 3D bounding boxes, or PostGIS solids for spatial queries over slabs and columns?
- **Multi-project transactions:** are they needed for portfolio or program workflows?
- **Read replicas and subscriptions:** is `NOTIFY` sufficient, or is logical replication the long-term change feed?
- **Offline store:** SQLite schema parity, or a log-only local file?
