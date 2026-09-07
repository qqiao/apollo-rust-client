# Apollo client specifications

These feature specifications describe what application developers and operators need from the existing Apollo client, with observable requirements and acceptance scenarios. They were reconstructed on 2026-09-06 from revision `4473cffe7a8a45c86978053da23ef36553b0ea3d`. The manifest says 0.7.0; this checkout also contains work listed as Unreleased. The scope is the checkout, not a claim about a published package.

**Status: Draft, pending maintainer acceptance.** The user selected Spec Kit-style documentation. Individual requirements and inferred priorities have not thereby received product approval.

## Feature specifications

| Feature | User outcome | Depends on |
|---|---|---|
| [001 — Read typed configuration](001-read-configuration/spec.md) | Select the correct application/rollout configuration, consume its values, and diagnose failures | Apollo configuration service |
| [002 — Retain configuration](002-retain-configuration/spec.md) | Reuse configuration through slow responses, outages, and restarts; warm and refresh it explicitly | 001 |
| [003 — Observe updates](003-observe-updates/spec.md) | Receive changes and control automatic refresh through the application's lifetime | 001, 002 |
| [004 — JavaScript client](004-javascript-client/spec.md) | Use these capabilities from browser and Node-style JavaScript applications | 001, 002, 003 |
| [005 — Real Apollo integration testing](005-real-apollo-testing/spec.md) | Reproduce client compatibility checks against automatically initialized real Apollo instances locally and in GitHub Actions | 001, 002, 003, 004 |

Feature 005 was requested on 2026-09-06 as a prospective capability following the retrospective reconstruction above. Its [technical plan](005-real-apollo-testing/plan.md), [implementation tasks](005-real-apollo-testing/tasks.md), and real-server test suites are now implemented and verified in-tree.

Read in numerical order. These boundaries follow consumer capabilities; they do not propose source modules or an implementation schedule. Each story can be demonstrated independently with the underlying dependency contracts/fixtures available; “independent” does not mean implementing a subscription without any retrieval mechanism.

Each `spec.md` uses prioritized stories, Given/When/Then scenarios, functional requirements, conceptual entities, and measurable outcomes, following the structure of GitHub's [Spec Kit feature template](https://github.com/github/spec-kit/blob/main/templates/spec-template.md). This is a format adaptation inside the requested `spec/` directory, not an installation of Spec Kit or a claim that a Specify CLI workflow/feature branch was executed.

## Supporting material

- [Contracts](supporting/contracts.md): environment variables, public Rust/JavaScript boundaries, Apollo wire behavior, and errors.
- [Technical design](supporting/design.md): current implementation choices, project structure, commands, and engineering constraints. These choices are descriptive, not mandatory feature requirements.
- [Research and decisions](supporting/research.md): provenance, conflicting evidence, suspected defects, unresolved policies, and reconstruction assumptions.
- [Traceability](supporting/traceability.md): requirement → scenario → existing evidence, with coverage limitations.
- [Requirements checklist](checklists/requirements.md): content/traceability review and verification results.

The older `specs/` directory in git history contained early software design descriptions. It remains historical reference material, with discrepancies recorded in research. The current `spec/` directory replaces that earlier draft; none of that draft is an approved baseline.

## Interpretation and ongoing SDD use

“MUST” identifies a proposed normative requirement once accepted. The implementation is evidence for reconstruction, not permission to freeze every existing behavior. Undocumented or suspect behaviors live in research and do not silently expand the contract. Source-only evidence is distinguished from test evidence; neither substitutes for maintainer acceptance of product intent.

Identifiers are local to a feature: reference `001-read-configuration/FR-001`, `002-retain-configuration/AC-001`, etc. Keep identifiers stable when updating a feature. P1/P2 is reconstructed importance/dependency, not historical prioritization or a future release commitment.

For a future change, revise the affected requirements/scenarios first, resolve relevant decision-register items, and review the intended behavior. Then derive the technical plan and implementation tasks, add or update meaningful acceptance tests, implement, and update evidence/user documentation. No fictitious implementation backlog or completed-plan history is generated for code that already exists.
