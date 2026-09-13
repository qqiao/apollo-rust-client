# Planning validation and acceptance coverage

**Date:** 2026-09-13. **Scope:** validate this specification/plan/task handoff, not implement or verify the future fixes.

## Coverage check

The seven packages contain 46 local functional requirements, 47 acceptance scenarios, and 22 success criteria. All local FR/AC/SC declarations are unique and consecutively numbered per package. Original IDs in features 001–005 were preserved; the appended scoped amendments are prospective and explicitly marked pending.

| Package | Scenarios | Task ownership / decisive evidence |
|---|---|---|
| 006 | AC-001/002 | T01: actual get_value path with suspended storage, completed refresh, returned/retained winner, no discarded-value event |
| 006 | AC-003–006 | T02: four named persisted-path companion tests; remote-only coalescing is insufficient evidence |
| 006 | AC-007 | T02: existing cancelled-initial-load regression, extended only if needed; T03 records all mappings |
| 007 | AC-001–004 | T01 external consumer tests + T02 actual default/Rustls all-target execution |
| 007 | AC-005 | T01 snapshot case and public rustdoc; T03 contracts/traceability |
| 008 | AC-001–004 | T01: marked actual Markdown snippet and generated-package ownership/error-path smoke |
| 008 | AC-005/006 | T02–T04: current primary/secondary/language examples; actual WASM format suite; T05 evidence |
| 009 | AC-001/004/006 | T01/T02: shared supervisor, held child, cached cleanup result, no source-text copying |
| 009 | AC-002/003/005/007 | T02: exit/status/log/signal/no-owned-run fault matrix and stderr fallback |
| 009 | AC-008 | T03: actual recovery CLI with stub Docker; shared helper's timeout; T04 real run; T05 evidence |
| 010 | AC-001/002/005 | T01: actual held headers/body, real abort, contextual secret-free timeout |
| 010 | AC-003/004/006 | T02: response/transport compatibility and timer disposal |
| 010 | AC-007 | T03: real fixture setup/verify/idempotency and all runtimes |
| 011 | AC-001–004/007 | T01: four explicit canonical snippets compiled as public consumers; negative controls; T02 translations |
| 011 | AC-005 | T02/T04: source-accurate API/type/tooling facts and limited current-status correction |
| 011 | AC-006 | T03a/b: eight actual missing language switches and local-file validation |
| 011 | AC-008 | T05: both checkers in both fast paths with propagated failures; T06 evidence |
| 012 | AC-001–004 | T01/T02: cadence/jitter/latency/backoff claim-to-source audit |
| 012 | AC-005/006 | T03 language audit; T04 final no-behavior-change evidence |

## Consistency checks performed on the handoff

- [x] Each finding has spec.md, plan.md, and tasks.md, with context, scope, dependencies, concrete files, acceptance criteria, and verification.
- [x] Source symbols, current scripts, supported test commands, test feature selectors, and error hierarchy were rechecked.
- [x] The review's imprecise DeserializeError wording was corrected: valid nested JSON/YAML variants must remain; their placement/public documentation is the issue.
- [x] Existing `scripts/test.sh fast` takes no filter, and future-only check commands are explicitly labeled as planned.
- [x] Package 007's external consumer tests must actually execute under Rustls; default-only external coverage is not mistaken for both configurations.
- [x] Package 009 specifies original stage/signal precedence, teardown failure, diagnostic failure, idempotence, real timeout supervision, and explicit recovery separately.
- [x] Package 010 preserves tolerant fixture HTTP result semantics while extending timer ownership through body completion; it does not change production client deadlines.
- [x] Package 011 distinguishes current executable snippets, historical migration examples, schematics, local file links, and external/heading links.
- [x] Package 012 explicitly forbids adding healthy jitter or an unverified latency bound to resolve a documentation defect.
- [x] No task or implementation checkpoint is pre-marked complete. Checkmarks in this file describe planning validation only.
- [x] New/changed planning-document local file links were checked; all resolved. This does not mark the eight existing wiki links fixed.
- [x] `git diff --check` and explicit trailing-whitespace/fence checks for new Markdown passed.
- [x] Git scope inspection shows only spec/ Markdown additions/changes; no application code, executable test, script, dependency, or generated artifact was changed.

## Verification limits and executor cautions

No Rust/WASM/Docker test suite was rerun during this documentation-only planning task. The historical review results in the index are baseline evidence, not proof of the planned fixes. Each executor must establish its own red/green or compiler/link/generated-package evidence.

Line numbers drift; identify the named functions before editing. The preferred regression seam and helper names are explicit implementation guidance, not a demand to preserve a demonstrably incompatible detail. If actual code changed, make the smallest justified adjustment that still meets the spec, document it, and do not weaken the expected outcome.

The original specs' general approval/status and broader D-* decisions remain unchanged. Task-level implementation authorization should come from the user's later request to execute the selected package; no external publication/merge/deployment is implied by this handoff.
