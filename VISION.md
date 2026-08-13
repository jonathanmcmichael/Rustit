# Vision

Rustit is an open-source BIM authoring tool and construction data platform built around three equal truths: what the building is, how it is made, and how those facts change over time.

## The premise

Building elements should not belong to an authoring application. Activities should not belong to a scheduling vendor. The relationship between a wall and the work that creates it should not disappear when a project changes tools.

Rustit therefore owns an open, IFC-based project model with stable identities for BIM objects, schedule objects, classifications, and their relationships. Desktop authoring, 4D visualization, database persistence, APIs, vendor synchronization, and AI all operate through the same deterministic model.

## Principles

1. **Authoring is primary.** Rustit must create and edit building information, not merely view exported files.
2. **BIM and scheduling are peers.** Schedule management and 4D links are part of the core project model, not a visualization plug-in.
3. **Users own their data.** No proprietary application or hosted service is required to fully access a project.
4. **Identity is stable.** Every project object has a durable, typed UUID independent of vendor identifiers.
5. **IFC is the semantic foundation.** IFC 4.3 concepts shape the canonical model; IFC files are serializations of that meaning, not a secondary export afterthought.
6. **Classifications are first-class.** MasterFormat, UniFormat, and future systems attach through IFC-aligned, edition-aware references.
7. **Vendors are adapters.** P6, Oracle Primavera Cloud, Bentley SYNCHRO, and future systems connect at defined boundaries.
8. **Geometry is replaceable.** A geometry kernel is an implementation behind an abstraction, not the architecture of the application.
9. **The model is headless.** Core operations work without launching the desktop UI.
10. **Operations become auditable.** Model and schedule changes will be proposed, validated, diffed, committed, and reversible.
11. **AI uses the same API.** AI may propose transactions but never bypasses validation, policy, or review.
12. **Open interoperability is a product.** IFC and documented APIs are first-class engineering work, not export checkboxes.

## What success looks like

A contributor can clone Rustit, create a building element, schedule its construction, persist both in an open database, inspect the 4D relationship, and exchange data with another system without surrendering the canonical project model.

The long-term ambition is a credible open alternative to proprietary BIM authoring and 4D platforms. The near-term discipline is to deliver small vertical slices that prove the architecture.
