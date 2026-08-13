# Rustit

**Open building modeling, built in Rust.**

[![CI](https://github.com/jonathanmcmichael/Rustit/actions/workflows/ci.yml/badge.svg)](https://github.com/jonathanmcmichael/Rustit/actions/workflows/ci.yml)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL--2.0-blue.svg)](LICENSE)

Rustit is an experimental, Rust-first BIM authoring tool with scheduling and 4D built into the project model. It asks what construction software could look like if building objects, schedule activities, and the links between them had open, stable identities instead of belonging to whichever application created them.

Rustit is **not currently a Revit replacement**. The first milestone is intentionally small enough to finish and understand.

## v0.0.1: It Opens

The current scaffold proves a narrow foundation:

- a desktop executable opens a native window;
- a semantic `Level` and `Wall` exist independently of the UI;
- an `Activity` and `ActivityRelationship` form an open schedule model;
- `ElementActivityLink` makes 4D a first-class relationship;
- a small CPM engine calculates early/late timings and critical path in working-hour units;
- PostgreSQL and vendor integration are expressed as interfaces, not embedded assumptions; and
- all domain objects use strongly typed UUID identities that survive serialization.

The window does not render the wall yet. That end-to-end semantic-to-geometry-to-rendering slice is the next milestone.

## Try it

Install a current stable Rust toolchain, then run:

```sh
cargo run -p rustit-app
```

Validate the complete workspace with:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
```

## Workspace

| Crate | Responsibility |
| --- | --- |
| `rustit-geometry` | Kernel-neutral geometry primitives and contracts |
| `rustit-model` | Semantic BIM authoring model (`Level`, `Wall`) |
| `rustit-schedule` | Activities, relationships, and deterministic CPM |
| `rustit-core` | Unified project and vendor-independent 4D links |
| `rustit-postgres` | PostgreSQL persistence interfaces and optimistic-version contract |
| `rustit-adapters` | IFC, P6, OPC, SYNCHRO, and future adapter boundaries |
| `rustit-app` | Minimal native desktop shell |

The dependency direction is deliberate: domain crates know nothing about the windowing toolkit, PostgreSQL driver, or vendor APIs.

## Vendor neutrality

P6, Oracle Primavera Cloud, Bentley SYNCHRO, and future scheduling systems are adapters around Rustit's open schedule. IFC and future model formats are adapters around the BIM model. PostgreSQL/PostGIS is the planned shared persistence implementation, not the definition of the domain.

An external system may call the same activity `A1040`, `781992`, or something else. Rustit keeps its own UUID and records the external identity at the adapter boundary.

## Project direction

- [Vision](VISION.md)
- [Architecture](ARCHITECTURE.md)
- [Roadmap](ROADMAP.md)
- [RFC 0001: The Rustit Project Model](docs/rfcs/0001-project-model.md)
- [Contributing](CONTRIBUTING.md)

Started as a joke. Built as a serious open-source experiment.
