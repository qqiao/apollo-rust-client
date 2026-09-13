# Feature Specification: Reliable current public documentation

**Feature ID:** `011-executable-public-docs`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R6 / P2, plus concrete adjacent documentation inconsistencies and broken links from the review.
**Dependencies:** [007](../007-public-cache-errors/spec.md), [008](../008-wasm-ownership-docs/spec.md), and 010's completed fast-runner edits; baseline 001–005 contracts.

## Purpose and scope

Application developers should be able to copy designated current Rust examples, identify actual error types, and navigate documentation without encountering nonexistent APIs or local pages. Maintainers need mechanical checks tied to the examples themselves so hand-maintained test copies cannot drift independently.

In scope: current configuration/import/error examples, actual error taxonomy, known false feature/tooling claims, translated equivalents, eight broken local language links, and bounded local example/link checks. Out of scope: rewriting the full wiki, executing historical migration examples as current code, adding unimplemented APIs to make erroneous prose true, changing runtime policies, translating entire missing guides, and checking every external URL online.

## Stories and acceptance scenarios

### Story 1 — Copy supported Rust examples (P1)

As a developer, I want current configuration and error examples that compile against the public crate.

**Independent test:** Extract explicitly marked current Rust fences from actual Markdown into a temporary consumer crate and compile them with Clippy.

- **AC-001:** Given the current native configuration example, when copied into its documented function-body context and compiled, then it constructs ClientConfig through the builder without missing struct fields.
- **AC-002:** Given the recommended public import example, when compiled, then it uses `client_config::ClientConfig` and any documented `CacheError` name only after 007 is implemented; no root ClientConfig re-export is invented to repair the prose.
- **AC-003:** Given the current error guide, when its typed example is compiled, then it matches actual top-level/cache/namespace error paths and distinguishes snapshot errors accurately.
- **AC-004:** Given a required marker/snippet is removed, duplicated, malformed, or edited to use an unavailable API, when the docs check runs, then it fails with document/marker context rather than passing zero examples.

### Story 2 — Read consistent, navigable guidance (P2)

As a reader, I want language variants and design references to agree with current implementation.

**Independent test:** Inspect the listed discrepancy inventory and run a local file-link checker over repository documentation.

- **AC-005:** Given error/feature/tooling descriptions, when read as current behavior, then they describe actual symbols and capabilities rather than nonexistent NamespaceNotFound or incorrectly placed DeserializeError, automatic IP detection, per-namespace secrets, or a Docker-free default full test command.
- **AC-006:** Given English/Simplified-Chinese design pages with language switches to absent Traditional-Chinese pages, when navigated, then every offered local page exists; do not add empty placeholder pages merely to silence the checker.
- **AC-007:** Given an explicitly historical “before” migration block or a clearly labeled partial schematic snippet, when documentation checks run, then it is not misrepresented as an executable current example; current examples cannot be relabeled historical merely to avoid fixing them.
- **AC-008:** Given a newly broken local file link or missing required example, when fast mode runs, then it fails visibly without Docker. Existing checks remain.

## Functional requirements

- **FR-001:** Designated current Rust examples MUST compile against the actual public crate under default native TLS and Rustls, without executing network requests.
- **FR-002:** Current configuration examples MUST avoid incomplete struct literals presented as complete code; use the existing builder for full examples.
- **FR-003:** Current error/import documentation MUST use existing public paths and explain actual conversion/snapshot limitations.
- **FR-004:** Example verification MUST extract the canonical Markdown content, require a nonempty explicit inventory, and fail on missing/duplicate/malformed required markers or compilation errors.
- **FR-005:** Local documentation links within the defined scope MUST resolve to existing files; intentional external/fragment-only links MUST not be confused with missing files.
- **FR-006:** Historical/illustrative material MUST remain clearly labeled and separate from the current executable inventory.
- **FR-007:** Translations of edited claims MUST preserve meaning, while short translated guides may link to detailed English examples rather than duplicate a full guide.
- **FR-008:** Fast checks MUST execute the new example/link validations, propagate failures, and preserve their Docker-independent operation.
- **FR-009:** Documentation corrections MUST NOT require new runtime capabilities, a dependency upgrade, or a whole-repository formatting rewrite.

## Entities and success criteria

**Entities:** canonical current snippet/marker, explicit snippet inventory, temporary consumer crate, current error taxonomy, local file link, historical example.

- **SC-001:** Required native-config/environment-config/public-imports/public-errors snippets pass both native configurations, and deliberate missing-marker/unavailable-symbol controls fail usefully.
- **SC-002:** All eight known local language-switch links are corrected and the repository-scope checker finds no missing file targets.
- **SC-003:** The discrepancy checklist in the plan is closed or explicitly justified against source, and edited translated claims agree.
- **SC-004:** Fast entry points include both checkers; no generated consumer crate is committed and tests use no Apollo network requests.

## Assumptions

Mechanical compilation initially covers a selected explicit inventory, not every Markdown fence. Link validation covers file existence, not external site availability or Markdown heading-anchor semantics. The original historical record should not be rewritten to suggest prior releases had future fixes. See [plan](plan.md) and [tasks](tasks.md).
