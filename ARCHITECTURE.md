# Architecture

Rustit is a modular Rust workspace organized around an open project model. The desktop app, persistence implementations, and vendor connectors depend on domain contracts; the domain does not depend on them.

```text
                         rustit-app
                              |
                        rustit-core
                       /           \
            rustit-model          rustit-schedule
                 | \                    / |
                 |  ----- rustit-ifc ----  |
                 |                       |
         rustit-geometry          CPM calculation

     rustit-postgres              rustit-adapters
             \                         /
              implement boundaries around
                    the project model
```

## Domain boundaries

### `rustit-ifc`

Defines the IFC 4.3.2.0 schema target, IFC-rooted 128-bit identity, the entity families used by the current model, and edition-aware classification references. It is a semantic foundation used by BIM, scheduling, and 4D—not merely an exporter. A serialization adapter will encode UUIDs as 22-character IFC `GlobalId` values and emit STEP or another IFC serialization.

### `rustit-geometry`

Defines project-coordinate primitives, wall geometry inputs, mesh output, and the `GeometryKernel` trait. It does not know what a BIM `Wall` is. Truck, Open CASCADE, or another provider can implement the contract after an evaluation RFC.

### `rustit-model`

Defines semantic authoring objects. In v0.0.1 these are `Level` (`IfcBuildingStorey`), `Wall` (`IfcWall`), and `BimModel`. A wall owns meaning, parametric inputs, and IFC classification associations; it does not own a window, database row, or vendor handle.

### `rustit-schedule`

Defines `Activity` (`IfcTask`), `ActivityRelationship` (`IfcRelSequence`), relationship kinds, and a small deterministic CPM calculation. The initial engine uses working-hour offsets. Calendars, data dates, constraints, actuals, resources, baselines, and date conversion remain future domain work.

### `rustit-core`

Defines `Project` (`IfcProject`) as the aggregate joining a BIM model, schedule, and `ElementActivityLink` collection. The link aligns with `IfcRelAssignsToProcess`; this is where vendor-independent 4D exists.

### `rustit-postgres`

Defines an asynchronous repository port with optimistic versions and non-secret connection settings. It intentionally has no database driver or schema in v0.0.1. A later implementation will target PostgreSQL and PostGIS while local/offline storage remains possible.

### `rustit-adapters`

Defines import, export planning, and export application contracts. Adapters declare capabilities because not every vendor API can safely support every operation. `ExternalIdentity` maps a Rustit UUID to a vendor-owned identifier without allowing that identifier into core domain types.

### `rustit-app`

Creates a native window and builds a tiny in-memory project. It is the composition root, never the owner of the semantic model.

## Identity

Domain IDs are strongly typed wrappers around a shared `IfcRootId`, backed by a UUID. They prevent accidental cross-domain substitution and remain losslessly convertible to IFC `GlobalId` values. Vendor identifiers are stored separately as external mappings.

## Classification

`ClassificationReference` mirrors the intent of IFC classification associations: it records a system, edition, identification, and optional title. MasterFormat and UniFormat are known systems, while an extension case supports future or project-specific classifications. Classification datasets are versioned outside the codebase; Rustit does not redistribute their copyrighted tables.

New objects currently receive random UUID v4 values. Import adapters must resolve a previously stored external mapping before creating a new UUID, preventing duplicate canonical objects across syncs.

## Dependency rules

- Domain crates must not depend on `rustit-app`, a GUI toolkit, a database driver, or a vendor SDK.
- `rustit-model` may depend on geometry primitives, but `rustit-geometry` must not depend on BIM semantics.
- Schedule vendors implement `ScheduleAdapter`; their concepts do not expand the canonical model without an accepted RFC.
- IFC defines semantic alignment across the model. IFC STEP is an interchange adapter, while Rustit's editing and transaction structures need not mirror a physical file graph.
- Write-capable sync must present a plan before applying external changes.

## Not yet implemented

Rendering, geometry generation, database schemas, live synchronization, IFC serialization, full classification catalogs, calendars, authentication, transactions, and AI interfaces are roadmap work. The interfaces in this scaffold reserve clean seams; they are not claims of working integrations.
