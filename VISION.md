# Vision

## Build the project, not another silo

Rustit is an open-source BIM authoring tool and construction data platform built around three equal truths:

- **what** the building is;
- **when and how** it is constructed; and
- **why the project changed.**

Today those truths are commonly divided among authoring applications, schedule systems, 4D products, databases, and spreadsheets. Each tool assigns its own identifiers, preserves only part of the project, and becomes authoritative by default. Moving between them means exporting, matching, repairing, and losing context.

Rustit starts from a different premise: **the project should be the source of truth, not the application that last edited it.**

The long-term goal is a credible open alternative to proprietary BIM authoring and 4D platforms. The immediate goal is smaller and more useful: prove, one vertical slice at a time, that a building object, the work that creates it, and the history connecting them can live in one open and deterministic model.

## The project graph

A wall is not only a solid. It has identity, location, type, materials, classifications, properties, relationships, and history. Construction is not only a finish date. It has activities, logic, calendars, constraints, progress, responsibility, and uncertainty. Four-dimensional planning is the relationship between those two models—not an animation assembled after both are finished.

Rustit treats the project as a connected graph:

```text
Building objects ── 4D links ── Schedule activities
       │                              │
       ├── IFC meaning                ├── logic and CPM
       ├── geometry                   ├── calendars and status
       ├── properties                 └── external schedule IDs
       └── classifications
                  │
          transactions and history
                  │
      desktop · database · API · AI
```

The same canonical object should survive authoring, scheduling, persistence, synchronization, and automation. Vendor identifiers can be attached to it, but they do not replace it.

## What Rustit is

Rustit is intended to become:

- a real BIM authoring application, not only an IFC viewer;
- an open schedule authoring and CPM environment, not only a schedule importer;
- a native 4D workspace where model-to-activity relationships are first-class data;
- a headless project engine usable from a desktop app, CLI, service, script, or agent;
- an open persistence and collaboration layer built around PostgreSQL and PostGIS;
- a documented integration platform for IFC, P6, Oracle Primavera Cloud, Bentley SYNCHRO, and future systems; and
- a safe foundation for auditable automation and AI-assisted project work.

Rustit is experimental. It is not currently a Revit, P6, or SYNCHRO replacement, and it should not pretend otherwise. Credibility comes from working vertical slices, tested interoperability, and explicit limitations.

## The product principles

### 1. Authoring is primary

Open construction software must be able to create and edit project meaning. Viewing proprietary exports is useful, but it leaves ownership and authorship elsewhere. Rustit must support the full path from an authored semantic object to generated geometry, persistence, scheduling, and exchange.

### 2. BIM and scheduling are peers

The schedule is not metadata attached to a finished model, and the model is not decoration attached to a finished schedule. `Wall`, `Activity`, `ActivityRelationship`, and `ElementActivityLink` belong to the same project model. This makes construction sequence, work packaging, status, and 4D reasoning possible without appointing a vendor product as the canonical database.

### 3. IFC supplies shared meaning

IFC 4.3 is Rustit's semantic foundation. Building objects, spatial structure, tasks, sequence relationships, classifications, and process assignments should have explicit paths to IFC concepts.

That does not mean using a STEP file as the in-memory editing engine. Rustit may use ergonomic Rust types, transactions, indexes, and regeneration structures while preserving IFC identity and meaning. IFC serialization is a tested boundary, not a checkbox added at the end.

### 4. Identity outlives every integration

Every canonical project object receives a stable, strongly typed UUID. P6 activity IDs, OPC identifiers, SYNCHRO IDs, database keys, and IFC encodings are mappings at system boundaries.

Stable identity is the prerequisite for dependable synchronization, history, collaboration, issue tracking, and AI. Without it, every exchange becomes another probabilistic matching exercise.

### 5. Classification is operational data

MasterFormat, UniFormat, and future classification systems should be edition-aware, source-aware associations—not loose text fields. Classifications connect design intent to specifications, estimating, work packages, procurement, and schedule structure.

Rustit stores project references and provenance without redistributing licensed classification catalogs.

### 6. Vendors are adapters, not owners

P6, Oracle Primavera Cloud, Bentley SYNCHRO, IFC files, and future systems connect through capability-aware adapters. Import translates external data into canonical objects and preserves external identity. Export is planned, reviewed, and applied deliberately.

An adapter may be incomplete because an external API is incomplete. That limitation must remain at the boundary rather than distorting the core project model.

### 7. Geometry is powerful and replaceable

Rustit needs a genuine geometry kernel, but a kernel is not the semantic architecture. The first implementation uses the pure-Rust Truck ecosystem behind a neutral contract. Other implementations can be compared or added as the project encounters building-scale topology, tolerance, exchange, or performance needs.

Meshes are products for rendering and exchange. They are not the authored meaning of a wall.

### 8. The core is headless

Creating a wall, calculating a schedule, validating a relationship, or committing a transaction must not require a desktop window. The graphical application is one client of the project engine.

This enables command-line workflows, testing, server deployments, automation, integrations, and alternative interfaces without rebuilding the domain.

### 9. Changes are transactions

Serious project software needs more than save and undo. Operations should become typed transactions that can be validated, previewed, diffed, approved, committed, audited, and reversed.

The same mechanism should eventually support local editing, database collaboration, vendor synchronization, and automated changes.

### 10. AI proposes; the project decides

AI should be able to query the project, identify inconsistencies, draft schedule logic, suggest classifications, generate alternatives, and propose changes. It must use the same typed APIs and validation rules as every other client.

AI does not receive direct, privileged mutation access. A useful construction agent must be inspectable and accountable: what it read, what it proposed, why validation passed or failed, who approved it, and what ultimately changed.

### 11. Open interoperability is part of the product

Interoperability requires stable identities, documented mappings, validation fixtures, conflict behavior, versioned schemas, and honest capability declarations. It is ongoing engineering work—not a collection of export buttons.

### 12. Small vertical slices beat broad promises

Rustit will earn its ambition by completing narrow paths through the whole architecture:

```text
semantic object
      ↓
validated parameters
      ↓
generated geometry
      ↓
rendered and selected
      ↓
scheduled and linked
      ↓
persisted and exchanged
```

Each slice should be understandable, tested, and useful before the next layer of breadth is added.

## The experience we are aiming for

A user creates a level and draws a wall. Rustit gives both stable identities, generates geometry, and records the edit as a transaction. The wall carries IFC meaning and project classifications.

The user creates or imports a schedule activity and links the wall to it as construction work. Rustit calculates the schedule, shows the 4D relationship, and preserves any P6, OPC, or SYNCHRO identity as an external mapping.

A rule, collaborator, or AI agent notices that the activity sequence conflicts with the model or project plan. It proposes a change with a readable explanation and diff. The user approves it. Rustit commits the transaction and can synchronize the result through adapters without losing canonical identity.

The desktop experience should make that workflow approachable. The underlying model should make it dependable.

## What makes this different

Rustit's wager is not merely that an existing BIM product can be rewritten in Rust. The deeper wager is that BIM authoring, scheduling, 4D, open data, and trustworthy automation can share one foundation.

The differentiators are structural:

- IFC-aligned semantics from the beginning;
- model and schedule objects with equal status;
- explicit, durable 4D relationships;
- vendor-neutral canonical identity;
- classification and provenance in the type system;
- a replaceable geometry layer;
- headless deterministic operations;
- open persistence and APIs; and
- one auditable transaction path for people, integrations, and AI.

Rust is a means to that end. Its type system, memory safety, performance, concurrency model, and cross-platform tooling are well suited to a project engine expected to remain dependable as its responsibilities grow. The product thesis is still about construction information, not the programming language.

## How success will be measured

Rustit succeeds when a contributor can:

1. clone the repository and understand the architecture;
2. create and edit semantic building objects;
3. see those objects regenerated and rendered;
4. author or import a real schedule and calculate it deterministically;
5. connect building elements to activities and inspect the 4D result;
6. save and reopen the project through open persistence;
7. exchange a documented IFC slice without losing identity or meaning;
8. synchronize a schedule through a vendor adapter with a reviewable plan; and
9. let an automated client propose a valid, auditable transaction through the same public API.

At a larger scale, success means teams can choose tools without surrendering ownership of the project graph, and an ecosystem can build on Rustit without asking permission from a single vendor.

## Near-term discipline

The project begins with deliberately modest milestones:

- **v0.0.1 — It Opens:** a clean workspace, native shell, semantic wall and level, open schedule objects, CPM, stable identity, 4D links, and boundary interfaces.
- **v0.0.2 — We Have Walls:** author a straight wall, generate its geometry, render it, select it, and keep the semantic object as the source of truth.
- **v0.0.3 — Open Schedule:** expand schedule authoring, calendars, status, constraints, and basic 4D playback.
- **v0.0.4 — It Persists:** store and reopen the unified project through PostgreSQL/PostGIS with explicit concurrency behavior.
- **v0.0.5 — It Interoperates:** complete the first identity-preserving IFC slice and a reviewable schedule adapter.

The names are playful. The exit conditions are not. Dungeon Crawler Carl has first claim on harmless milestone aliases, demo projects, and synthetic fixtures; [the Easter-egg policy](EASTER_EGGS.md) keeps that personality out of canonical construction semantics.

## An invitation

Rustit is for people who believe construction software can be open without being vague, interoperable without being lowest-common-denominator, and AI-enabled without making project changes unaccountable.

The project needs more than Rust developers. It needs architects, engineers, builders, schedulers, VDC practitioners, computational designers, database engineers, IFC specialists, geometry experts, and people who have spent too many hours repairing handoffs between systems.

The repository is public because the architecture, tradeoffs, failed experiments, and progress should be inspectable. The vision is ambitious. The implementation will remain honest.

**Build the project, not another silo.**
