# Rustit Truth Labs

These files are small, synthetic, MPL-2.0-licensed reference scenarios. They encode independently readable expectations for parallel contribution lanes and are executed by `crates/rustit-fixtures/tests/truth_labs.rs`.

| Lab | Current truth |
| --- | --- |
| Wall Lab | Authored dimensions produce known bounds and 12 triangles; invalid dimensions are rejected |
| Schedule Lab | All four CPM relationship types and one lead produce known working-hour timings |
| 4D Lab | Stable element and activity identities form a vendor-neutral process assignment |
| IFC Lab | Current Rustit semantic types map to the declared IFC 4.3 ADD2 entity families |
| Sync Lab | A canonical UUID remains separate from a vendor-owned external identifier |

## Fixture rules

- Keep each scenario small enough to calculate or inspect manually.
- State units, time basis, tolerance, identity, and expected failures explicitly.
- Never derive expected values by copying Rustit output into the fixture without independent review.
- Use deterministic identifiers so diffs and round trips remain readable.
- Do not contribute customer data, proprietary model files, credentials, or licensed classification catalogs.
- Add a fixture or extend an existing one when a domain behavior changes.

Run only these contracts with `cargo xtask labs`, or run the complete repository gate with `cargo xtask verify`.
