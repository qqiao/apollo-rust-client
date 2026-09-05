---
trigger: always_on
---

# Mandatory Spec-Driven Development

All agents working on this project MUST use spec-driven development (SDD).
Specifications define intended behavior; code and tests provide implementation
evidence. Do not substitute an implementation inventory for a specification.

## 1. Check specifications before working

- At the start of every task, read `spec/README.md` and the feature specifications
  relevant to the request. Recheck them when the scope changes.
- Identify the applicable feature IDs, functional requirements (`FR-*`), acceptance
  scenarios (`AC-*`), and success criteria (`SC-*`). Read dependency specifications
  and supporting contracts or unresolved decisions when they affect the task.
- Use `spec/` for SDD requirements.
- Check specification status. A draft or an observation of current code is not
  evidence of maintainer approval. Preserve decisions and authorization already
  supplied by the user; do not repeatedly request the same approval.

## 2. Specify before implementing

- Before adding or changing behavior, update the affected specification. If no
  feature covers the request, create `spec/<feature-id>/spec.md` and add it to the
  index before writing implementation code.
- Follow the project's Spec Kit-style format: purpose and scope, prioritized user
  stories, independent tests, Given/When/Then acceptance scenarios, functional
  requirements, key entities, measurable success criteria, and assumptions.
- Describe observable user outcomes. Keep implementation choices, architecture,
  source/test mappings, and execution evidence in separate supporting documents.
- Preserve stable feature and requirement IDs. Reference requirements with their
  feature ID so that locally numbered IDs are unambiguous.
- Resolve material ambiguities or conflicting intended behavior with the user
  before implementing dependent changes. Continue independent work where possible.
  Do not silently promote a suspected defect or unresolved decision into a requirement.
- Do not weaken a requirement or acceptance scenario merely to fit existing code
  or make a failing check pass.

## 3. Plan and derive tasks

- Follow this order: **specification → technical plan → tasks → implementation →
  verification**. Do not backfill a specification after implementing a new behavior.
- Base the plan on the relevant requirements. Give each implementation task its
  acceptance scenarios and verification method.
- For substantial changes, record the plan and tasks alongside the feature as
  `plan.md` and `tasks.md`, unless an existing project convention applies.
- Scale documentation to the change without skipping SDD. For a small fix,
  documentation edit, or behavior-preserving refactor, an existing specification
  and a brief written plan/checklist are sufficient if they fully cover the work.
  Do not invent product requirements for mechanical edits.
- Read-only investigation and review still begin with the spec check; report
  findings against the contract without fabricating implementation tasks.

## 4. Implement and verify against the contract

- Implement only the authorized scope covered by the specification and tasks.
  Update the specification first if that scope or intended behavior changes.
- Verify the applicable acceptance scenarios and success criteria. Add or update
  meaningful regression tests for behavior changes; distinguish automated tests,
  source inspection, and unverified assumptions.
- Follow `.agent/rules/rust.md`, including `scripts/test.sh` as the test entry point
  and Clippy instead of cargo check. Use appropriate documentation checks for
  documentation-only changes.
- Keep affected specifications, contracts, traceability, and user documentation
  consistent with the result. Record discovered gaps rather than hiding them.
- Before claiming completion, report the specifications/requirement IDs addressed,
  the verification performed, and any failures or remaining gaps. Never claim
  approval, passing tests, or satisfied acceptance criteria without evidence.
