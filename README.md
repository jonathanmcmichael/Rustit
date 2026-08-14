# Rustit

**Open building modeling, built in Rust.**

[![CI](https://github.com/jonathanmcmichael/Rustit/actions/workflows/ci.yml/badge.svg)](https://github.com/jonathanmcmichael/Rustit/actions/workflows/ci.yml)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL--2.0-blue.svg)](LICENSE)

Rustit is an experimental, Rust-first, IFC-based BIM authoring tool with scheduling and 4D built into the project model. It asks what construction software could look like if building objects, schedule activities, classifications, and the links between them had open, stable identities instead of belonging to whichever application created them.

Rustit is **not currently a Revit replacement**. The first milestone is intentionally small enough to finish and understand.

## v0.0.1: It Opens

The current scaffold proves a narrow foundation:

- a desktop executable opens a native window;
- a semantic `Level` and `Wall` exist independently of the UI;
- an `Activity` and `ActivityRelationship` form an open schedule model;
- `ElementActivityLink` makes 4D a first-class relationship;
- the model targets IFC 4.3.2.0 concepts such as `IfcBuildingStorey`, `IfcWall`, `IfcTask`, `IfcRelSequence`, and `IfcRelAssignsToProcess`;
- elements and activities accept IFC-aligned MasterFormat and UniFormat classification references;
- a small CPM engine calculates early/late timings and critical path in working-hour units;
- PostgreSQL and vendor integration are expressed as interfaces, not embedded assumptions; and
- all domain objects use strongly typed UUID identities that survive serialization.

Development has begun on v0.0.2. The desktop app now takes its semantic `Wall`, asks the replaceable Truck kernel for a neutral triangle mesh, and renders that mesh with `wgpu` over a ground plane. The first wall is no longer a hardcoded cube. Authoring controls, selection, and camera navigation remain next.

## Try it

Install a current stable Rust toolchain on a machine with a WebGPU-compatible graphics adapter, then run:

```sh
cargo run -p rustit-app
```

Validate the complete workspace with:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
```

Contributors can run the same complete gate as CI with `cargo xtask verify`. The smaller `cargo xtask labs` command executes the synthetic Wall, Schedule, 4D, IFC, and Sync truth fixtures.

## Workspace

| Crate | Responsibility |
| --- | --- |
| `rustit-geometry` | Kernel-neutral geometry primitives and contracts |
| `rustit-geometry-truck` | Experimental Truck B-rep, tessellation, Boolean, and STEP implementation |
| `rustit-ifc` | IFC schema target, rooted identity, entities, and classifications |
| `rustit-model` | Semantic BIM authoring model (`Level`, `Wall`) |
| `rustit-schedule` | Activities, relationships, and deterministic CPM |
| `rustit-fixtures` | Executable synthetic truth labs for domain and interoperability behavior |
| `rustit-core` | Unified project and vendor-independent 4D links |
| `rustit-postgres` | PostgreSQL persistence interfaces and optimistic-version contract |
| `rustit-adapters` | IFC, P6, OPC, SYNCHRO, and future adapter boundaries |
| `rustit-app` | Native desktop shell and neutral-mesh GPU renderer |
| `xtask` | One-command repository verification runner |

The dependency direction is deliberate: domain crates know nothing about the windowing toolkit, PostgreSQL driver, or vendor APIs.

## IFC-based, not IFC-washed

[IFC 4.3.2.0](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html) is Rustit's semantic baseline. `Level`, `Wall`, `Activity`, activity relationships, and 4D assignment links state their corresponding IFC entity families. Rustit keeps an ergonomic, transaction-friendly Rust representation rather than pretending a STEP exchange file is an authoring engine; IFC import/export serializes that model without inventing a separate proprietary meaning.

[MasterFormat](https://crmservice.csinet.org/widgets/masterformat/numbersandtitles.aspx?id=f369fc97-bed5-ea11-80f3-000d3a04ff75) and [UNIFORMAT II](https://www.nist.gov/publications/uniformat-ii-elemental-classification-building-specifications-cost-estimating-and-cost) are modeled as edition-aware `IfcClassificationReference`-style associations. Rustit uses `UniFormat` as the internal system name and accepts project-specific labels. It does not bundle copyrighted classification tables. Projects and adapters supply the applicable edition and preserve its code, title, and source.

## Vendor neutrality

P6, Oracle Primavera Cloud, Bentley SYNCHRO, and future scheduling systems are adapters around Rustit's open, IFC-aligned schedule. IFC STEP and future serializations are adapters around the IFC-based authoring model. PostgreSQL/PostGIS is the planned shared persistence implementation, not the definition of the domain.

An external system may call the same activity `A1040`, `781992`, or something else. Rustit keeps its own UUID and records the external identity at the adapter boundary.

## Project direction

- [Vision](VISION.md)
- [Architecture](ARCHITECTURE.md)
- [Roadmap](ROADMAP.md)
- [RFC 0001: The Rustit Project Model](docs/rfcs/0001-project-model.md)
- [RFC 0002: Truck Geometry Kernel Evaluation](docs/rfcs/0002-truck-geometry-kernel-evaluation.md)
- [Contributing](CONTRIBUTING.md)

## Put your AI energy to work

Rustit is designed for parallel, evidence-backed contribution. Start with an [`ai-ready` work packet](https://github.com/jonathanmcmichael/Rustit/issues?q=is%3Aissue%20state%3Aopen%20label%3Aai-ready), give it to the coding workflow you trust, and return a narrow pull request whose result can be reproduced by humans and CI.

- [AI-assisted contribution policy](AI_CONTRIBUTING.md)
- [Agent contract](AGENTS.md)
- [Work-packet specification](docs/contributing/WORK_PACKETS.md)
- [Definition of Done](docs/contributing/DEFINITION_OF_DONE.md)
- [Contribution infrastructure roadmap](docs/CONTRIBUTION_ROADMAP.md)
- [Community evidence scoreboard](docs/community/SCOREBOARD.md)
- [Governance](GOVERNANCE.md) and [maintainers](MAINTAINERS.md)

The project measures merged packets, fixtures, prevented regressions, round trips, and returning contributors. Prompt counts, token totals, and generated lines are not evidence of a better construction tool.

Started as a joke. Built as a serious open-source experiment.

## The most important architectural priority

**Dungeon Crawler Carl is the primary Easter-egg canon.** New projects preferentially receive spoiler-free codenames such as **Princess Donut**, **Carl**, **Mongo**, **Mordecai**, **The Royal Court**, and **Safe Room**. The deterministic demo project is Princess Donut territory, its first level is Dungeon Floor 1, and completing v0.0.1 remains an achievement worth unlocking. See [EASTER_EGGS.md](EASTER_EGGS.md) for the rules and the [publisher's series page](https://www.penguinrandomhouse.com/series/43C/dungeon-crawler-carl/) for the actual books.

The earlier **Space King**, **Aqua Teen Hunger Force**, and **The Big Lez Show** references—including **Sassy the Sasquatch**, **Big Lez**, **Mike Nolan**, **Clarence**, and **Donny**—remain in the rotation. These are affectionate cultural references in synthetic names and UI flavor, not project dependencies, copied dialogue, or endorsements.
