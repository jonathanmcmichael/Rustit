# AI-Assisted Contributions

Rustit welcomes contributions made with Codex, Claude, Copilot, local models, and future coding systems. The project evaluates the resulting change and its evidence—not whether every character was typed manually.

## Human accountability

Every pull request has a human submitter who is responsible for:

- understanding the intended behavior and the material parts of the diff;
- checking generated claims against source code, standards, fixtures, or cited primary documentation;
- ensuring the contribution contains no confidential, credentialed, proprietary, or improperly licensed material;
- running the stated verification and reporting failures honestly; and
- responding to review rather than delegating responsibility to the model.

Prompt transcripts and token counts are not required. Identify material AI assistance in the pull request so reviewers understand how the change was produced. Never include secrets or project data in that disclosure.

## Evidence over confidence

An AI-generated explanation is not acceptance evidence. Prefer, in order:

1. deterministic automated tests;
2. small human-readable domain fixtures with known results;
3. round-trip comparisons or invariants;
4. reproducible benchmarks;
5. visual evidence for interactive behavior; and
6. a clearly labeled manual verification when automation is not yet practical.

Tests must assert independently known behavior. A model must not derive an expected result from the same implementation being tested and present that agreement as validation.

## Safe agent scope

Agents should receive the least access needed for one work packet. They must not:

- push directly to `main`;
- merge their own work;
- bypass failing checks or weaken a gate to obtain a pass;
- invent IFC compliance, vendor capability, construction policy, or classification content;
- introduce a new dependency without explaining why existing code is insufficient; or
- expand a bounded issue into a speculative subsystem.

If an agent discovers a larger architectural question, preserve the minimized evidence and open an RFC or follow-up issue.

## Useful disclosure

In the pull request, state:

- which AI tools materially assisted;
- what the human reviewed or independently verified;
- any uncertain or generated portions that deserve extra attention; and
- whether inputs were exclusively synthetic or public.

The disclosure is for review quality, not attribution theater. Contributions are accepted because the repository can understand, reproduce, and maintain them.

## Synthetic naming

When generated examples need arbitrary names, follow [EASTER_EGGS.md](EASTER_EGGS.md): Dungeon Crawler Carl references have first priority, stay spoiler-free, and never replace the explicit domain meaning. Agents may name a fixture `princess-donut-straight-wall`; they may not rename `IfcWall`, `ActivityRelationship`, or a vendor-owned field.
