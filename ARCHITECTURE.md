# Architecture

Rustit is a modular Rust workspace organized around an open project model. The desktop app, persistence implementations, and vendor connectors depend on domain contracts; the domain does not depend on them.

```text
                         rustit-app
                              |
                        rustit-core
                       /           \
            rustit-model          rustit-schedule
                 |                       |
         rustit-geometry          CPM calculation

     rustit-postgres              rustit-adapters
             \                         /
              implement boundaries around
                    the project model
```

## Domain boundaries

### `rustit-geometry`

Defines project-coordinate primitives, wall geometry inputs, mesh output, and the `GeometryKernel` trait. It does not know what a BIM `Wall` is. Truck, Open CASCADE, or another provider can implement the contract after an evaluation RFC.

### `rustit-model`

Defines semantic authoring objects. In v0.0.1 these are `Level`, `Wall`, and `BimModel`. A wall owns meaning and parametric inputs; it does not own a window, database row, or vendor handle.

### `rustit-schedule`

Defines `Activity`, `ActivityRelationship`, relationship kinds, and a small deterministic CPM calculation. The initial engine uses working-hour offsets. Calendars, data dates, constraints, actuals, resources, baselines, and date conversion remain future domain work.

### `rustit-core`

Defines `Project` as the aggregate joining a BIM model, schedule, and `ElementActivityLink` collection. This is where vendor-independent 4D exists.

### `rustit-postgres`

Defines an asynchronous repository port with optimistic versions and non-secret connection settings. It intentionally has no database driver or schema in v0.0.1. A later implementation will target PostgreSQL and PostGIS while local/offline storage remains possible.

### `rustit-adapters`

Defines import, export planning, and export application contracts. Adapters declare capabilities because not every vendor API can safely support every operation. `ExternalIdentity` maps a Rustit UUID to a vendor-owned identifier without allowing that identifier into core domain types.

### `rustit-app`

Creates a native window and builds a tiny in-memory project. It is the composition root, never the owner of the semantic model.

## Identity

Domain IDs are UUID-backed newtypes such as `ElementId` and `ActivityId`. They are strongly typed to prevent accidental cross-domain substitution and serializable so a round trip preserves identity. Vendor identifiers are stored separately as external mappings.

New objects currently receive random UUID v4 values. Import adapters must resolve a previously stored external mapping before creating a new UUID, preventing duplicate canonical objects across syncs.

## Dependency rules

- Domain crates must not depend on `rustit-app`, a GUI toolkit, a database driver, or a vendor SDK.
- `rustit-model` may depend on geometry primitives, but `rustit-geometry` must not depend on BIM semantics.
- Schedule vendors implement `ScheduleAdapter`; their concepts do not expand the canonical model without an accepted RFC.
- IFC implements a model adapter and informs semantic design, but Rustit's editing and transaction model is not required to mirror an IFC file graph.
- Write-capable sync must present a plan before applying external changes.

## Not yet implemented

Rendering, geometry generation, database schemas, live synchronization, IFC translation, calendars, authentication, transactions, and AI interfaces are roadmap work. The interfaces in this scaffold reserve clean seams; they are not claims of working integrations.
