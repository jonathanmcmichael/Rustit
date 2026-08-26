# Classification provenance (Wall Lab)

## Provenance

This fixture supplies **project-level classification references only**. It does not redistribute MasterFormat, UniFormat, or any other licensed classification table.

| Field | Meaning |
| --- | --- |
| `system` | Named classification system (`MasterFormat` or `UniFormat`) |
| `edition` | Project-supplied edition label (`project edition`) — not a CSI/NIST catalog drop |
| `identification` | A single code the project already chose |
| `name` | Optional human title retained with the reference |

Truth source (issue #20): synthetic references `07 00 00 / Thermal and Moisture Protection` and `B2010 / Exterior Walls`, both with edition `project edition`.

## Licensing limits

- Do **not** expand this file into a classification catalog, number-and-title dump, or searchable code list.
- Catalog download, licensed validation against vendor data, and specification integration are out of scope.
- Projects and adapters must obtain full catalogs through appropriately licensed sources outside Rustit.

## Invariants exercised by `truth_labs`

1. Both references round-trip with every field intact (including wall identity).
2. Adding an identical reference twice yields one association.
3. A blank identification is rejected.