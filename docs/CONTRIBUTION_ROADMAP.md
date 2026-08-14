# Contribution Infrastructure Roadmap

The product roadmap describes what Rustit should do. This roadmap describes how a growing community can build it without losing architectural coherence.

## Stage 1 — Establish the foundry

- Land the initial project, geometry, vision, and rendering stack on `main`.
- Publish governance, maintainer, conduct, security, AI-assistance, and work-packet policies.
- Standardize issue intake, pull-request evidence, review ownership, and labels.
- Protect `main` with pull requests and required checks.

Exit condition: a newcomer can discover who decides, what is safe to change, and how completion is judged.

## Stage 2 — Encode project truth

- Add Wall, Schedule, 4D, IFC, and Sync Labs with synthetic fixtures.
- Provide `cargo xtask verify` as the common local and CI gate.
- Add contract tests, deterministic snapshots, geometry benchmarks, and supported-platform checks incrementally.
- Turn accepted RFC questions and kernel graduation gates into work packets.

Exit condition: independent teams can change different lanes while shared fixtures detect semantic regressions.

## Stage 3 — Grow parallel lanes

- Maintain ready queues for model, geometry, app, scheduling, IFC, 4D, persistence, adapters, and docs.
- Pair Rust reviewers with domain reviewers for changes that need both.
- Publish a recurring evidence scoreboard.
- Run bounded public challenges whose results merge through normal work packets.

Exit condition: useful contributions arrive from people who are not already repository experts.

## Stage 4 — Distribute stewardship

- Recognize recurring domain reviewers.
- Add maintainers through the public governance process.
- Assign code ownership by active review area.
- Document release, deprecation, incident, and compatibility policies before production claims.

Exit condition: project throughput and judgment do not depend on one person or one AI system.

## Measures

Track outcomes, not activity theater:

- ready packets completed;
- fixtures added and regressions prevented;
- median time from ready issue to first review;
- contributors returning for a second merged change;
- domains with an active reviewer;
- deterministic round trips and benchmark scenarios; and
- unresolved architecture or security risks.

Do not rank contributors by prompt count, token usage, generated lines, or raw commit volume.
