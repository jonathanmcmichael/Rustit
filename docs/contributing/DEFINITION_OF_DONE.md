# Definition of Done

A Rustit contribution is done when a reviewer can understand why it exists, reproduce its evidence, and maintain it without recovering missing intent from an AI chat.

## Every pull request

- Links a ready issue or explains why an issue was unnecessary.
- States the user/domain outcome and explicit non-goals.
- Preserves the dependency rules in `ARCHITECTURE.md` and `AGENTS.md`.
- Includes focused tests or explains the narrow reason automation is impractical.
- Uses only synthetic, public, or explicitly redistributable inputs.
- Documents material AI assistance, new dependencies, and external assumptions.
- Passes `cargo xtask verify` when available.
- Leaves no unexplained generated files, credentials, customer data, or licensed catalogs.

## Domain behavior

- Expected results are independently known and readable by a domain reviewer.
- Units, tolerance, time basis, identity, and classification edition are explicit where relevant.
- Invalid inputs and failure behavior are tested, not only the happy path.
- IFC and vendor claims name the implemented slice and do not imply broader conformance.

## Interactive behavior

- Core logic is tested separately from window or GPU plumbing where practical.
- A reproducible manual check and visual evidence cover behavior CI cannot see.
- Keyboard, mouse, scaling, resize, and error behavior are considered when relevant.

## Architecture or dependencies

- Boundary changes have an accepted RFC.
- New dependencies include purpose, maintenance, license, and removal or containment considerations.
- Vendor-specific concepts remain behind capability-aware adapters.

Meeting the checklist does not guarantee merge. It ensures review begins from evidence rather than archaeology.
