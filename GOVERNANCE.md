# Governance

Rustit uses lightweight, repository-first governance while the community is forming. Decisions, evidence, and accepted tradeoffs belong in issues, pull requests, and RFCs rather than private chats.

## Roles

### Participant

Anyone who reports a reproducible problem, reviews domain behavior, improves a fixture, joins a design discussion, or helps another contributor.

### Contributor

A participant with a merged contribution. Contributions include code, tests, fixtures, documentation, reviews, issue decomposition, and standards expertise.

### Domain reviewer

A contributor trusted to review a defined area such as BIM semantics, project controls, IFC, geometry, graphics, persistence, security, or contributor experience. Domain reviewers do not need to be Rust specialists and do not receive merge authority automatically.

### Maintainer

A contributor responsible for repository health, issue readiness, architectural coherence, releases, and merges. Maintainers are expected to combine technical judgment with respectful community stewardship.

Current maintainers and review areas are recorded in `MAINTAINERS.md`.

## Decisions

- A routine, bounded change is decided through pull-request review and passing gates.
- Changes to the project model, public APIs, identity, persistence format, geometry-kernel policy, transactions, adapter contracts, or dependency direction require an RFC.
- Security reports follow `SECURITY.md` and may be handled privately until disclosure is safe.
- When evidence is incomplete, the decision may remain provisional with explicit graduation gates.

The bootstrap maintainer has final merge responsibility until the maintainer group grows. That authority does not override accepted RFCs or required checks; exceptions must be documented publicly.

## Growing responsibility

Roles are earned through sustained evidence rather than application volume or generated lines of code.

A domain reviewer should demonstrate several of the following:

- accurate, constructive reviews in a named domain;
- useful fixtures or minimized failure cases;
- reliable follow-through on issues and review feedback;
- respect for project boundaries and confidential-data rules; and
- an ability to distinguish known behavior from assumptions.

A maintainer candidate should also demonstrate cross-area judgment, release discipline, community support, and dependable review availability. Existing maintainers nominate and publicly record new reviewers and maintainers. Maintainer inactivity does not erase credit; access may be adjusted when sustained availability changes.

## Conflicts and appeals

State disagreements in terms of user outcome, accepted principles, fixtures, and tradeoffs. If a contributor disagrees with a review decision, they may request an RFC or ask another relevant reviewer to weigh in. Conduct concerns follow `CODE_OF_CONDUCT.md`.

## Changes to governance

Material governance changes use a pull request with at least seven days for public comment once Rustit has more than one active maintainer. During bootstrap, the maintainer may merge an urgent clarification sooner but must explain why.
