# RFC 0001: The Rustit Project Model

- Status: Accepted for the initial scaffold
- Authors: Jonathan McMichael and initial contributors
- Created: 2026-08-13

## Summary

Rustit has one canonical `Project` that contains a semantic BIM model, an open schedule, and explicit links between building elements and activities. Stable typed UUIDs identify objects. Geometry engines, databases, IFC, and schedule vendors remain replaceable implementations at the boundary.

## Motivation

AEC workflows commonly split the same project among authoring, scheduling, 4D, GIS, and spreadsheet systems. Each system assigns its own identity and treats its representation as authoritative. The result is repeated matching, opaque sync behavior, and fragile ownership of project data.

Rustit needs an authoring model that can answer both “what is this building object?” and “when and why is work performed on it?” without requiring a vendor application to be the source of truth.

## Decision

### Project aggregate

The initial aggregate is:

```text
Project
├── ProjectId
├── BimModel
│   ├── Level
│   └── Wall
├── Schedule
│   ├── Activity
│   └── ActivityRelationship
└── ElementActivityLink
```

The `Project` validates references that cross domain boundaries. For example, a 4D link cannot be added unless its element and activity exist.

### Stable identity

Each persistent object has a domain-specific UUID newtype. A UUID is generated when Rustit creates a new object and preserved through serialization and persistence. Strong newtypes prevent using an activity ID where an element ID is expected.

Imports consult an external-identity mapping before creating a new UUID:

```text
Rustit object UUID  <->  external system + external ID
```

Vendor IDs never replace canonical Rustit IDs.

### BIM authoring

`Level` and `Wall` are the first semantic types. A wall stores its level reference, baseline, thickness, and height. These inputs generate geometry; a stored render mesh is not the meaning of the wall.

### Scheduling and CPM

`Activity` and `ActivityRelationship` are canonical schedule types. Finish-start, start-start, finish-finish, and start-finish relationships are supported at the model boundary. The initial CPM engine works in abstract working-hour units and rejects cyclic networks.

Calendars, constraints, resources, WBS, baselines, actuals, and date anchoring require later RFCs or compatible additions. Vendor fields must not be added merely because one integration exposes them.

### 4D

`ElementActivityLink` connects an `ElementId` to an `ActivityId` and declares a role such as construct, demolish, temporary, or inspect. It belongs to the project model, not to P6, OPC, SYNCHRO, or a renderer.

### Geometry

The geometry crate defines primitives and a `GeometryKernel` contract. The semantic model may request wall geometry, but no kernel implementation owns the semantic wall. Selection of Truck, Open CASCADE, or another approach requires evaluation and a separate RFC.

### Persistence

Application code saves and loads `Project` through a repository port. The first shared implementation is expected to use PostgreSQL and PostGIS. The domain remains usable in memory and must not depend on a database connection.

Optimistic project versions form the initial concurrency boundary. Transactions, object history, branches, and merging require dedicated design before implementation.

### Adapters

Schedule systems implement `ScheduleAdapter`; model interchange systems implement `ModelAdapter`. Import produces canonical objects plus identity mappings and warnings. Export is split into planning and application so callers can review changes before an external write.

Initial named targets are:

- Oracle Primavera P6
- Oracle Primavera Cloud
- Bentley SYNCHRO
- IFC
- future schedule and model systems through the same contracts

The list is directional, not a claim that integrations exist in v0.0.1.

### AI

AI is a client of deterministic project APIs. Future AI operations must produce proposed transactions that can be validated, diffed, approved, and audited. AI does not receive a privileged mutation path.

## Consequences

- BIM and scheduling can evolve independently while meeting in the project aggregate.
- The UI can be replaced or supplemented by CLI, server, scripting, and AI clients.
- Database and vendor integrations can be developed without contaminating core types.
- Some concepts will be modeled twice at system boundaries and require explicit mapping.
- The project accepts upfront design work to avoid vendor-shaped domain objects.

## Rejected alternatives

### Make IFC the complete internal editing graph

IFC is a first-class interoperability target, but its exchange graph does not by itself define Rustit's editing transactions, schedule model, history, or application invariants.

### Make PostgreSQL the domain model

A schema is an implementation and operational concern. Domain behavior must remain testable and usable without a running database.

### Use P6 or SYNCHRO IDs as canonical identities

Vendor IDs are neither universal nor stable across every import/export path. Rustit must retain identity when a project changes tools.

### Add 4D after BIM authoring is mature

Deferring 4D would allow the BIM object model and schedule model to evolve without a durable relationship. The link is small enough to establish immediately.

## Unresolved questions

- How are units, coordinate reference systems, and tolerances represented?
- What is the smallest deterministic transaction and dependency model?
- Which local/offline persistence format complements PostgreSQL?
- How are calendars and time zones represented across schedule vendors?
- Which geometry kernel best supports the first wall-authoring slice?
- What subset of IFC can round-trip without semantic loss first?
- How are permissions and approvals attached to external sync operations?

These questions should become focused RFCs as their vertical slices begin.
