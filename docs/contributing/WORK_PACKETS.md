# AI-Ready Work Packets

A work packet is a bounded problem that a domain expert can hand to a human-and-AI implementation team without transferring unwritten assumptions. It is the unit of parallel contribution in Rustit.

## Definition of ready

An issue may receive `state:ready` and `ai-ready` when it contains:

1. **Outcome:** the user or domain result, stated without prescribing a large design.
2. **Truth source:** a synthetic fixture, primary reference, or independently known expected behavior.
3. **Invariants:** architectural and domain rules the change must preserve.
4. **Scope:** expected crates or documents and the smallest useful boundary.
5. **Non-goals:** tempting adjacent work that is explicitly excluded.
6. **Acceptance:** objective checks a reviewer can reproduce.
7. **Evidence:** tests, round trips, benchmark, screenshot, or documented manual check.
8. **Dependencies:** blocking issues, RFCs, or merged capabilities.
9. **Size:** `size:xs`, `size:s`, or `size:m`.
10. **Review lane:** at least one `review:*` label when domain review is material.

If any required fact is uncertain, label the issue `state:needs-domain` rather than asking an agent to invent it.

## Size guide

| Size | Expected shape |
| --- | --- |
| `size:xs` | One file or fixture, one behavior, usually under half a day for an experienced contributor |
| `size:s` | One crate or narrow cross-file slice, usually one to two focused days |
| `size:m` | A complete vertical slice with several contracts, expected to require explicit coordination |

Larger work should become an RFC plus multiple packets.

## Packet lifecycle

```text
needs-domain -> proposed -> ready -> in-progress -> in-review -> done
                       \-> blocked
```

- A contributor comments with the smallest outcome they intend to deliver.
- Maintainers use issue assignment only to reduce duplicate effort, not to create permission barriers.
- A stale claim may be released after fourteen days without an update.
- Discoveries outside scope become linked issues instead of expanding the active pull request.
- Merging the pull request closes the packet and updates relevant fixtures or the scoreboard.

## Example

> Change Wall Lab A from 3.0 m to 4.2 m. Preserve its UUID and baseline, regenerate the neutral mesh to a 4.2 m maximum elevation, reject invalid heights without mutation, and introduce no Truck types into `rustit-model`.

That statement names an observable result, stable-identity and dependency invariants, and a deterministic geometry check. It can be implemented with different tools without changing what counts as correct.
