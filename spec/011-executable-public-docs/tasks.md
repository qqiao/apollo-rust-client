# Tasks: Executable public docs

Status: Completed and verified.

## T01 — Correct canonical Rust examples and compile actual Markdown

- [x] Complete T01.
- **Files (4):** `docs/wiki/en/Configuration.md`, `docs/wiki/en/Upgrade-Guide.md`, `docs/wiki/en/Error-Handling.md`, new `scripts/check-doc-examples.mjs`.
- **Contracts:** FR-001–004/006; AC-001–004/007; SC-001.
- **Work:** Replaced invalid struct/import examples with builder examples, corrected errors, marked the four canonical current fences, and implemented temporary consumer compilation across native-tls and rustls.
- **Acceptance:** Actual doc content compiles under both native configurations; missing/duplicate/invalid-marker controls fail; unavailable-symbol control fails compilation with doc context.
- **Verify:** `node --check scripts/check-doc-examples.mjs`; `node scripts/check-doc-examples.mjs`; negative tests in `tests/tooling/doc-examples.test.mjs`.

## T02 — Align translated current API guidance

- [x] Complete T02a.
- [x] Complete T02b.
- **Depends on:** T01.
- **Files per substep (3):** T02a: `docs/wiki/zh-CN/{Configuration,Upgrade-Guide,Error-Handling}.md`; T02b: corresponding zh-TW files.
- **Contracts:** FR-002/003/006/007; AC-002/003/005/007; SC-003.
- **Work:** Corrected current imports (`use apollo_rust_client::{client_config::ClientConfig, Client}`) and matching guidance; linked concise translated pages to canonical compiling code; preserved valid historical examples.
- **Acceptance:** Translations do not recommend unavailable APIs; history remains labeled.
- **Verify:** Contextual search/checklist against source, local links, `git diff --check`.

### Checkpoint A

- [x] Canonical examples are mechanically checked and translated guidance agrees.

## T03 — Implement local link validation and repair missing language switches

- [x] Complete T03a.
- [x] Complete T03b.
- **Depends on:** T01.
- **Files T03a (5):** new `scripts/check-doc-links.mjs`, `docs/wiki/en/Design-Overview.md`, `Design-WASM.md`, `Design-ClientConfig.md`, `Design-Client.md`.
- **Files T03b (4):** corresponding `docs/wiki/zh-CN/Design-*.md` pages.
- **Contracts:** FR-005/007; AC-006/008; SC-002.
- **Work:** Checked real local file links, removed eight switches pointing at nonexistent zh-TW pages, retained valid switches. Corrected known error-taxonomy inventory in Design-Overview.
- **Acceptance:** All scoped file links resolve; intentional external/fragment-only links remain excluded; controlled missing-file fixture fails with location/target.
- **Verify:** `node --check scripts/check-doc-links.mjs`; `node scripts/check-doc-links.mjs`; negative fixture tests in `tests/tooling/doc-links.test.mjs`; `git diff --check`.

## T04 — Correct concrete adjacent current claims

- [x] Complete T04a.
- [x] Complete T04b.
- **Depends on:** T02a/T02b and T03a/T03b.
- **Files T04a (4):** `docs/wiki/en/Features.md`, `docs/wiki/zh-CN/Features.md`, `docs/wiki/zh-TW/Features.md`, `CHANGELOG.md`.
- **Files T04b (5):** `src/lib.rs`, `src/cache.rs` (rustdoc only), `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, `spec/005-real-apollo-testing/spec.md` (status prose only).
- **Contracts:** FR-003/006/007/009; AC-005/007; SC-003.
- **Work:** Removed unsupported automatic-IP and per-namespace-secret claims, corrected full-versus-fast Docker requirement in CHANGELOG, narrowed listener lock-release wording in rustdoc and design docs, and separated historical planning text from feature-005 current status.
- **Acceptance:** No executable runtime behavior changed; stated capabilities match source; no historical verification/approval is invented.
- **Verify:** Source-to-doc discrepancy checklist; both doc checkers; `git diff --check`; source diff contains comments only for T04b.

## T05 — Make documentation checks part of fast verification

- [x] Complete T05.
- **Depends on:** T01–T04 and 010 completed.
- **Files (3):** `scripts/test.sh`, `scripts/apollo-test.sh`, `tests/apollo/README.md`.
- **Contracts:** FR-008; AC-008; SC-004.
- **Work:** Invoked both document checkers once in both fast paths (`scripts/test.sh` and `scripts/apollo-test.sh`), preserving all prior checks. Explained selected executable coverage and file-link-only scope in `tests/apollo/README.md`.
- **Acceptance:** Checker failure propagates; no Docker prerequisite is added; required snippet inventory cannot disappear into zero-test success.
- **Verify:** Shell syntax checks; `scripts/test.sh fast`; confirm checkers and default/Rustls consumer compilation appear in actual output.

## T06 — Close documentation traceability

- [x] Complete T06.
- **Depends on:** T05.
- **Files (4):** `spec/supporting/contracts.md`, `spec/supporting/research.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** all package FR/AC/SC.
- **Work:** Recorded corrected claims, exact checked snippet pairs, language review, link counts, negative controls, and remaining historical/unchecked-doc limits.
- **Acceptance:** Every AC has evidence; no claim all wiki fences or external URLs were validated; no generated package/manifest committed.
- **Verify:** Final `git diff --check`, doc checkers, scope review.

## Completion evidence

- **Check Inventory and Compilation (AC-001–004)**:
  - 4 canonical snippets verified and compiling across both `native-tls` and `rustls` with Clippy `-D warnings`:
    1. `native-config` (`docs/wiki/en/Configuration.md`): `ClientConfig::builder`
    2. `environment-config` (`docs/wiki/en/Configuration.md`): `ClientConfig::from_env`
    3. `public-imports` (`docs/wiki/en/Upgrade-Guide.md`): `use apollo_rust_client::{client_config::ClientConfig, Client}`
    4. `public-errors` (`docs/wiki/en/Error-Handling.md`): `apollo_rust_client::Error` matching with `CacheError` and `namespace::Error`
- **Negative Tooling Controls (AC-001, AC-004)**:
  - `tests/tooling/doc-examples.test.mjs` (8 tests): duplicate markers, unclosed fences, non-rust language, empty snippets, missing required inventory, and compiler failure diagnostics for nonexistent symbols.
- **Local Markdown Link Validation (AC-006, AC-008)**:
  - `scripts/check-doc-links.mjs`: Scanned 418 local Markdown links across 77 documents in `README.md`, `README_zh.md`, `CHANGELOG.md`, `docs/wiki/`, and `spec/`. 0 broken links.
  - `tests/tooling/doc-links.test.mjs` (4 tests): inline/reference link extraction, external/anchor exclusion, broken relative link detection, and fixture folder verification.
  - Repaired 8 broken language switches in `docs/wiki/{en,zh-CN}/Design-{Client,Overview,WASM,ClientConfig}.md` pointing to nonexistent `zh-TW/Design-*.md`.
- **Adjacent Claims and Discrepancies Corrected (AC-005, AC-007)**:
  - `docs/wiki/en/Features.md`: Clarified secret belongs to `ClientConfig` (not per-namespace); explicit client IP configuration (no automatic IP detection).
  - `docs/wiki/en/Configuration.md`: Corrected cache directory structure to `<base>/apollo-rust-client/config-cache/` and filename pattern to `v2-{sha1_digest}.cache.json`.
  - `CHANGELOG.md`: Clarified fast check suite is Docker-independent while full integration uses Docker.
  - `src/lib.rs` & `src/cache.rs` rustdocs + `docs/wiki/{en,zh-CN}/Design-Cache.md`: Narrowed listener lock release wording to memory and listener-list locks.
  - `docs/wiki/en/Design-Overview.md`: Corrected error taxonomy.
  - `docs/wiki/{zh-CN,zh-TW}/Upgrade-Guide.md`: Corrected public imports.
  - `spec/005-real-apollo-testing/spec.md`: Separated historical planning provenance from verified status.
- **Fast Path Integration (AC-008)**:
  - Both `scripts/check-doc-links.mjs` and `scripts/check-doc-examples.mjs` wired into `run_fast_checks` in `scripts/test.sh` and `scripts/apollo-test.sh`. Verified passing via `./scripts/test.sh fast`.
