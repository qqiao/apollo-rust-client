# Repair round 3: review feedback and clean-checkout verification

Leader-authored plan, 2026-09-13. Baseline: `4ae64a68a0f6c6b0f7ab20fd02f6702fd155db9b`.
Authorization: maintainer requested another repair round, internal review, then a fresh Copilot review. This is corrective work under existing 010/FR-006, 011/FR-004–005, 013/FR-007, and 014/FR-001–003, FR-008; it introduces no new client feature or dependency.

## Findings and implementation order

### R3-1 — Restore clean-checkout documentation validation

- CI run 34775130743, test job 103771748186: all 66 tooling tests pass, then six missing-target links fail validation. Build and quality jobs pass.
- The maintainer intentionally deleted HANDOFF.md and three transient review documents. Preserve those deletions. Repair the six references in 013 plans/specs, spec/README.md, review-remediation/README.md and verification-2026-09-13.md. Use durable current documentation or explicitly historical GitHub revision links where provenance matters; do not fabricate replacement review reports or empty placeholders.
- Acceptance: link checking succeeds from tracked files alone, with the deleted documents absent. Preserve the checker's scope and failure behavior. Verify no remaining live local references to deleted handoff/review artifacts.

### R3-2 — Cover common secret query naming forms

- In scripts/apollo-fixtures.mjs, extend the shared redaction policy to appSecret, clientSecret, apiKey, accessKey, authorization and equivalent case/underscore/hyphen forms, preserving existing sensitive names and ordinary query values (014/AC-003).
- Apply consistent policy to absolute URL objects/strings and relative or malformed fallback paths. Keep credentials redacted and never include the unredacted URL in timeout diagnostics.
- Acceptance: table-driven unit cases and real request timeout cases prove secrets absent, including duplicate sensitive query keys; ordinary host/path/query context remains. Run HTTP tooling tests before and after the fix.

### R3-3 — Parse balanced inline destinations

- Replace the first-closing-parenthesis matcher in scripts/check-doc-links.mjs with bounded linear scanning that tracks nesting, angle-delimited destinations and escapes. Keep title handling, reference definitions, external URLs, and fence exclusions effective.
- Acceptance: existing files with balanced parentheses, escaped parentheses and angle-bracket paths containing spaces/parentheses resolve through extractLinks and checkFileLinks; missing files still fail. Include malformed/unclosed inputs to prove termination and no spurious truncated target. Run documentation link regressions.

### R3-4 — Map compiler errors to source examples

- In scripts/check-doc-examples.mjs, associate generated binary names with snippet filePath, markerLine and id; report this context on compilation failures for either TLS configuration while preserving compiler diagnostics and nonzero failure.
- Acceptance: the unavailable-symbol negative control asserts actual source document, marker ID and line in captured diagnostics. Multiple snippets must not misattribute one binary's failure to another. Canonical examples still compile under both configurations.

### R3-5 — Reconcile operational documentation

- Correct the spawn hook name and LIFECYCLE_TEST_MODE gate in plan.md. Document spawning as deferral until child registration, followed by abort and cleanup; include spawning in the phase inventory.
- Make package 014 discoverable from the remaining handoff/index documents without restoring deleted HANDOFF.md. Label historical verification scope explicitly; avoid rewriting dated evidence as a current run. Replace stale current test totals with verified totals or count-independent acceptance criteria.
- Acceptance: source-to-document audit covers all nine suppressed review comments and the inline secret-redaction comment; each has an explicit disposition, including comments made obsolete by intentional deletions.

## Verification and delivery gates

- Implement R3-1 through R3-5 with test-driven development, preserving existing constraints, ownership checks, timeout contracts and assertions. No production Rust/WASM behavior changes.
- Checkpoint after R3-1/R3-2: focused links and HTTP regressions pass. After R3-3/R3-4: all documentation tooling and negative controls pass.
- Final checks: scripts/test.sh fast; Linux lifecycle tests; scripts/test.sh integration --suite native with scoped zero-resource teardown; git diff --check. Verify documentation against a tracked-files-only checkout so local artifacts cannot conceal missing targets.
- Coder performs code-review-and-quality, code-simplification, security-and-hardening and performance-optimization checks; reports per-finding dispositions, commands/results and exact pushed SHA. The Leader remains available as the doubt-driven-development adversary.
- Leader personally reviews the submitted changes and evidence; repeat fixes if needed. Only after internal review passes, request a fresh GitHub Copilot review and record its event and head SHA. Do not merge the PR. Do not wait for external CI unless explicitly requested.
