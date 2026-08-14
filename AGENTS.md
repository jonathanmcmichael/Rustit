# Rustit Agent Contract

This file is the operating contract for AI coding agents and humans directing them in this repository. Read `VISION.md`, `ARCHITECTURE.md`, and the relevant accepted RFCs before changing code.

## Mission in ten lines

1. Rustit is an open BIM authoring and construction-data platform.
2. The project graph—not the last application to edit it—is canonical.
3. BIM, scheduling, and durable 4D links are peer domains.
4. IFC 4.3 ADD2 supplies shared meaning and an exchange boundary.
5. Stable typed UUIDs outlive database rows and vendor identifiers.
6. MasterFormat, UniFormat, and other classifications retain system, edition, and provenance.
7. Geometry kernels are replaceable implementations behind neutral contracts.
8. P6, OPC, SYNCHRO, IFC files, and future systems are adapters, never owners of core types.
9. Humans, integrations, and AI must eventually propose the same validated transactions.
10. Small evidence-backed vertical slices outrank broad speculative subsystems.

## Non-negotiable boundaries

- Domain crates must not depend on `rustit-app`, `wgpu`, `winit`, a database driver, or a vendor SDK.
- `rustit-geometry` must not depend on BIM semantics. Kernel crates implement its contracts.
- Semantic objects are authored truth; meshes and solids are generated products.
- Vendor identifiers remain `ExternalIdentity` mappings at adapter boundaries.
- Do not replace stable IDs during editing, regeneration, persistence, or round trips.
- Do not claim IFC conformance until a documented fixture proves the relevant exchange slice.
- Write-capable synchronization must remain plan-then-apply.
- A boundary change requires an RFC before implementation is merged.

## Working method

1. Start from an issue labeled `state:ready` and `ai-ready`, or ask that the work be made ready.
2. Restate the acceptance checks and relevant invariants before editing.
3. Keep the change within one reviewable work packet. Do not opportunistically redesign adjacent crates.
4. Add or improve a deterministic fixture whenever domain behavior changes.
5. Run `cargo xtask verify` when available; until then run the commands in `CONTRIBUTING.md`.
6. Report uncertainty and failed experiments. Never turn an assumption into a passing assertion.
7. Open a pull request; never push directly to `main`.

## Data and provenance

- Use synthetic, anonymized, or explicitly redistributable fixtures only.
- Never submit customer/project data, credentials, proprietary model files, or licensed classification tables.
- Small classification references may appear in tests; catalogs must come from appropriately licensed sources.
- Record new external data, algorithms, generated assets, and dependencies in the pull request.
- AI assistance is welcome, but the submitting human owns the change and must review every line and claim.

## Easter-egg convention

- Dungeon Crawler Carl is the first-priority reference set for harmless demo, fixture, test, milestone, and codename labels.
- Prefer spoiler-free names such as Princess Donut, Carl, Mongo, Mordecai, The Royal Court, Dungeon Floor, Safe Room, and Achievement.
- Keep the plain construction meaning beside the reference, and follow [EASTER_EGGS.md](EASTER_EGGS.md).
- Never leak fandom terminology into canonical domain types, IFC mappings, persistence schemas, vendor adapters, or compliance claims.
- Do not copy dialogue, prose, artwork, logos, or audio. Names and brief allusions are enough.

## Stop and request review when

- the issue lacks an objective acceptance check;
- domain intent conflicts with an accepted RFC;
- a change crosses crate or vendor boundaries not named in the work packet;
- a fixture may contain confidential or licensed material;
- deterministic verification is impossible with the available evidence; or
- the implementation would require weakening validation, CI, identity, or auditability.
