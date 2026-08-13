# Contributing to Rustit

Rustit welcomes Rust developers, BIM and scheduling practitioners, CAD/geometry engineers, interoperability specialists, designers, and technical writers.

The project is experimental. Small, tested vertical slices are more valuable than large speculative subsystems.

## Before starting

1. Read [VISION.md](VISION.md), [ARCHITECTURE.md](ARCHITECTURE.md), and the accepted RFCs.
2. Open an issue or discussion before substantial architectural work.
3. Keep vendor-specific concepts behind adapter boundaries.
4. Preserve the IFC 4.3 semantic mapping and edition/source information on classification references.
5. Do not introduce a geometry kernel, database driver, UI framework expansion, or vendor SDK without documenting the tradeoff.

## Local development

Rustit uses stable Rust. Run these checks before opening a pull request:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Add focused tests for new domain behavior. Code that only declares future architecture without supporting the current vertical slice usually belongs in an RFC rather than a crate.

## Pull requests

- Explain the problem and the user or developer impact.
- Keep changes narrow enough to review.
- Call out new dependencies and external-system assumptions.
- Update an RFC or architecture document when a boundary changes.
- Never commit database URLs, API keys, model files, or customer/project data.
- Do not commit proprietary MasterFormat or UniFormat tables; use small references in tests and obtain catalogs through appropriately licensed sources.

## RFC process

Use an RFC for changes that affect the project model, public APIs, identity, persistence format, geometry kernel, transaction semantics, adapter contracts, or cross-crate dependency direction.

Copy the structure of [RFC 0001](docs/rfcs/0001-project-model.md), assign the next available number, and submit it as a pull request. An RFC should separate the problem, decision, consequences, rejected alternatives, and unresolved questions.

Until formal governance is adopted, the project maintainer makes final calls after public review. Accepted decisions should be recorded in the repository rather than left only in chat or issue threads.

## License

By contributing, you agree that your contributions are licensed under the Mozilla Public License 2.0.
