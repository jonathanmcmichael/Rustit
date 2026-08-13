# RFC 0002: Truck Geometry Kernel Evaluation

- Status: Provisionally accepted for the v0.0.2 prototype
- Authors: Jonathan McMichael and initial contributors
- Created: 2026-08-13

## Summary

Rustit will use [Truck](https://github.com/ricosjp/truck) as the first implementation of its kernel-neutral geometry contracts while keeping that implementation replaceable. Truck is good enough to carry the first semantic-wall-to-render-mesh vertical slice, but this evaluation does not declare it the permanent or exclusive Rustit geometry kernel.

The prototype lives in `rustit-geometry-truck`. The semantic `Wall` remains in `rustit-model`, geometry inputs and mesh output remain in `rustit-geometry`, and IFC serialization remains an adapter responsibility.

## Why evaluate an existing kernel

A BIM authoring application needs more than triangles. It needs repeatable parametric regeneration, boundary representation, openings, joins, intersections, tolerances, and exchange geometry. Reimplementing those foundations before Rustit can draw its first authored wall would delay the product experiment and create risk in the least differentiated layer.

Truck is a modular, Apache-2.0, Rust-native CAD platform with B-rep topology, parametric geometry, meshing, shape operations, and STEP support. It is therefore a strong first candidate that does not introduce a C++ foreign-function boundary.

## Evaluation slice

The prototype intentionally tests a small AEC-shaped slice:

1. Convert a validated straight, horizontal `WallGeometry` into a closed B-rep solid.
2. Tessellate that solid into Rustit's kernel-neutral `Mesh`.
3. Regenerate after changing a wall parameter and obtain deterministic results.
4. Create a rectangular through-opening as an authored inner boundary.
5. Evaluate experimental Boolean subtraction and wall union support.
6. Export a directly modeled wall as generic STEP geometry.

These checks are executable unit tests rather than screenshots or hand-edited samples.

## Results

| Capability | Result | Notes |
| --- | --- | --- |
| Straight wall B-rep | Pass | Translational sweeps produce a closed solid from baseline, thickness, and height. |
| Tessellation | Pass | Truck's meshing produces indexed triangles in Rustit's neutral mesh type. |
| Parametric regeneration | Pass | Changing height changes the regenerated extent; repeated identical inputs produce equal meshes. |
| Authored rectangular opening | Pass | An inner face boundary swept through the wall produces a closed solid. |
| Boolean rectangular opening | Deferred | Truck shape operations currently enable an unused VTK dependency chain with known vulnerable parsers. |
| Perpendicular L-wall Boolean union | Deferred | Boolean operations will return behind a reviewed dependency boundary. |
| Direct modeled-solid STEP output | Pass | `truck-stepio` emits a `MANIFOLD_SOLID_BREP` STEP document. |

The earlier experiment also found that Truck's Boolean union returned no result for the evaluated coplanar wall-junction case. That remains a material finding: wall joins contain coincident and coplanar faces that are common in BIM and difficult for Boolean engines. Rustit must not disguise that limitation behind a passing box test. The v0.0.2 wall slice can proceed without destructive joins, but secure dependency closure and robust join behavior are gates for broader authoring use.

## Decision

Adopt Truck provisionally as Rustit's first geometry implementation, behind the existing `GeometryKernel` boundary.

This means:

- v0.0.2 may use Truck for straight-wall solid generation and rendering tessellation;
- authored topology is preferred when the operation is naturally parametric, such as a rectangular wall opening;
- Truck shape operations stay outside the default build until their dependency path passes the repository security gate;
- the semantic model must never store Truck handles as canonical wall identity or meaning; and
- Rustit may add another kernel implementation or replace Truck without migrating the project model.

This does not mean:

- Truck types become part of `rustit-model` or `rustit-core`;
- generic STEP output is treated as IFC output;
- the failed L-wall join is acceptable for a mature authoring tool; or
- the current dependency versions are approved indefinitely.

## IFC and STEP boundary

Truck's STEP writer proves exchange of directly modeled geometric topology. It does not produce an IFC project, `IfcWall`, spatial containment, properties, classifications, or stable IFC-rooted identity. A future IFC adapter must map Rustit's semantic objects to IFC 4.3 entities and use kernel output only for the applicable representation items.

The current `truck-stepio` documentation also states that shapes created by set operations cannot be output. Rustit must therefore keep IFC authoring and exchange design independent of Truck's generic STEP writer.

## Dependency and maintenance risks

The retained versions are `truck-modeling 0.6.0`, `truck-meshalgo 0.4.0`, and `truck-stepio 0.3.0`.

Rustit disables `truck-meshalgo`'s default features because the project needs tessellation but does not yet expose VTK import or export. Truck's shape-operations crate currently re-enables that VTK chain, including vulnerable legacy XML and compression parsers, so it has been removed from the default dependency graph rather than allowlisted. The remaining graph reports a future-incompatibility warning for `nom 3.2.1` through STEP support; that must be resolved upstream, patched, or isolated before Rustit treats the kernel stack as production-ready.

Other risks include:

- shape-operation robustness on coincident and nearly coincident building geometry;
- tolerance policy across project scale, coordinates, and IFC import;
- maturity and release cadence of the Truck crate family;
- topology naming across regeneration for stable selection and constraints; and
- performance and memory behavior on building-scale models.

## Gates for broader adoption

Before Truck graduates from provisional status, Rustit should demonstrate:

- the v0.0.2 semantic-wall-to-window rendering slice;
- deterministic regeneration and selection mapping for edited walls;
- wall end, T, L, and crossing joins without silent invalid topology;
- multiple openings and openings near joins;
- a documented project-wide units and tolerance policy;
- IFC 4.3 representation mapping and round-trip fixtures;
- representative model benchmarks; and
- removal, upgrade, or explicit containment of future-incompatible dependencies.

Failure on one gate does not force the semantic model to change. It triggers comparison with another implementation, such as an Open CASCADE adapter, or a focused contribution upstream.

## Alternatives considered

### Build a Rust kernel from scratch

Rejected for the first vertical slice. It would make topology and numerical geometry the project before Rustit proves its BIM and 4D authoring thesis.

### Bind directly to Open CASCADE

Deferred as a valuable comparison and fallback. Open CASCADE has deeper industrial history, but brings a C++ build and deployment boundary. The neutral geometry crate keeps this option open.

### Store meshes as authored geometry

Rejected. Meshes are rendering and exchange products; they do not preserve the parametric intent of a wall, opening, or join.

### Make IFC STEP the live geometry kernel

Rejected. IFC defines building semantics and exchange representations, not Rustit's interactive solid-modeling algorithms, regeneration graph, or transaction behavior.

## Follow-up

The immediate next task is to connect a semantic `rustit-model::Wall` to `TruckGeometryKernel`, upload its neutral mesh to the renderer, and display it in the v0.0.2 app without moving Truck types into the domain model.
