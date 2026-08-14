# Roadmap

The roadmap favors complete vertical slices over broad collections of placeholders. Versions describe outcomes, not deadlines.

## v0.0.1 — It Opens · Achievement Unlocked

- [x] Native desktop window
- [x] Stable typed UUIDs
- [x] IFC 4.3.2.0 semantic baseline
- [x] MasterFormat and UniFormat classification references
- [x] Semantic `Level` and `Wall`
- [x] `Activity` and `ActivityRelationship`
- [x] Vendor-independent `ElementActivityLink`
- [x] Initial CPM forward/backward pass
- [x] PostgreSQL persistence contract
- [x] Vendor adapter contracts
- [x] Workspace tests and CI

Exit condition: a contributor can clone the repository, run the app, and see a window backed by a valid in-memory BIM-plus-schedule project.

## v0.0.2 — We Have Walls · Dungeon Floor 1

- [x] Evaluate Truck behind the kernel-neutral geometry contract
- Create and edit a level and straight wall through the app
- [x] Generate wall geometry through a kernel implementation
- [x] Render a ground plane and wall mesh
- Orbit, pan, zoom, select, and inspect
- Keep the semantic wall as the source of rendered geometry

## v0.0.3 — Open Schedule · Mordecai's Tutorial

- Author activities and all four relationship types
- Add work calendars, data date, actuals, and constraints
- Run CPM with clear validation and diagnostics
- Link elements to activities and play a basic 4D sequence
- Import/export a documented open schedule format

## v0.0.4 — It Persists · Safe Room

- Define and migrate the PostgreSQL/PostGIS schema
- Save and reopen the unified project model
- Preserve external identities and sync state
- Add transactions, optimistic concurrency, and object-level diffs
- Define a dependable local/offline workflow

## v0.0.5 — It Interoperates · The Royal Court

- Serialize the first IFC 4.3 authoring vertical slice without losing stable identity
- Prototype one read-only schedule adapter
- Compare imported data before committing it
- Export through capability-aware, reviewable sync plans

## Later

- Parametric dependencies and regeneration
- Drawings, annotations, schedules, and sheets
- Object-level history, branching, merging, and collaboration
- Cost, documents, GIS, fabrication, and field status
- Auditable AI and MCP interfaces using proposed transactions
- Additional adapters for P6, Oracle Primavera Cloud, Bentley SYNCHRO, and community-selected systems

Each milestone may change through the RFC process. The principles in [VISION.md](VISION.md) and dependency direction in [ARCHITECTURE.md](ARCHITECTURE.md) are the guardrails.

The parallel [Contribution Infrastructure Roadmap](docs/CONTRIBUTION_ROADMAP.md) defines how work is decomposed, verified, reviewed, and distributed across a growing human-and-AI contributor community.
