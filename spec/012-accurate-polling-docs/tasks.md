# Tasks: Accurate polling documentation

Status: Completed and verified.

## T01 — Correct public polling promises

- [x] Complete T01.
- **Files (3):** `README.md`, `README_zh.md`, `CHANGELOG.md`.
- **Contracts:** FR-001–005; AC-001–005; SC-001/002.
- **Work:** Replaced false healthy jitter/latency claims with completed-round cadence and scoped failure jitter. Kept full-response polling and absence of long-polling/releaseKey explicit; removed unsupported interval-only latency bounds.
- **Acceptance:** Both READMEs and Unreleased describe the same implemented behavior; TTL/request timeout are not conflated with poll interval; no runtime changes.
- **Verify:** Source-to-prose comparison; contextual search; `git diff --check`.

## T02 — Align technical reference wording

- [x] Complete T02.
- **Depends on:** T01.
- **Files (up to 5):** `spec/supporting/design.md`, `docs/wiki/en/Design-Client.md`, `docs/wiki/en/Design-Cache.md`, corresponding zh-CN Design-Client/Design-Cache.
- **Contracts:** FR-001–006; AC-001–006; SC-001/003.
- **Work:** Corrected cadence/backoff claims in design references; added technical formula for bounded failure backoff; preserved 006 restoration and 011 callback-lock wording.
- **Acceptance:** Numeric cap/jitter description is accurate for base intervals both below and above 300; explanations distinguish polling from manual/stale reads; no global ordering/rate/SLO guarantee added.
- **Verify:** Inspected actual function calculations (`refresh_delay_seconds`, `is_backing_off`); example/link checkers; `git diff --check`.

### Checkpoint A

- [x] Entry-point and technical polling guidance agree with source.

## T03 — Close remaining current-language occurrences

- [x] Complete T03a (English).
- [x] Complete T03b (Simplified Chinese).
- [x] Complete T03c (Traditional Chinese).
- **Depends on:** T02.
- **Files per substep (up to 3):** `Configuration.md`, `Features.md` across en, zh-CN, and zh-TW.
- **Contracts:** FR-004/005; AC-005/006; SC-002.
- **Work:** Inspected search results, aligned current claims regarding polling cadence (sleep after completed round), bounded concurrency (up to 4), and failure backoff.
- **Acceptance:** No current healthy-jitter or interval-only bound survives; translated meaning agrees; historical text is not used to evade correction.
- **Verify:** Contextual search, both documentation checkers, diff inspection for scope.

## T04 — Record the source-to-claim audit

- [x] Complete T04.
- **Depends on:** T03a–c.
- **Files (3):** `spec/supporting/research.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** all FR/AC/SC.
- **Work:** Recorded six scenario mappings, corrected documents, final checker results, and historical claim disposition. Marked the Unreleased polling discrepancy corrected without claiming runtime changes.
- **Acceptance:** Each AC has source/doc evidence; executable source/dependencies unchanged; all tasks complete.
- **Verify:** `git diff --check`, `node scripts/check-doc-examples.mjs`, `node scripts/check-doc-links.mjs`.

## Completion evidence

- **Claim-to-Source-to-Document Mapping**:
  - **AC-001 (Completed round cadence & max 4 concurrency)**:
    - *Source*: `src/lib.rs` (`refresh_loop`, `buffer_unordered(4)`, `platform_sleep(Duration::from_secs(base_interval))`).
    - *Documents*: `README.md`, `README_zh.md`, `CHANGELOG.md`, `docs/wiki/en/Design-Client.md`, `docs/wiki/zh-CN/Design-Client.md`.
  - **AC-002 (Healthy polling has no intentional jitter)**:
    - *Source*: `src/lib.rs` (`platform_sleep(Duration::from_secs(base_interval))` sleeps the un-jittered base interval).
    - *Documents*: `README.md`, `README_zh.md`, `CHANGELOG.md`, `spec/supporting/design.md`, `docs/wiki/en/Design-Client.md`, `docs/wiki/zh-CN/Design-Client.md`.
  - **AC-003 (Update latency not bounded by refresh_interval alone)**:
    - *Source*: Round duration $R$, network latency, server publication delay, callback execution time, and retry delays add to total observation time ($R + I$).
    - *Documents*: `README.md`, `README_zh.md`, `spec/supporting/design.md`.
  - **AC-004 (Failure backoff formula and scoped jitter)**:
    - *Source*: `src/cache.rs` (`refresh_delay_seconds`: nominal $\min(b \cdot 2^{\min(n, 4)}, \max(b, 300))$ with integer $\pm 10\%$ jitter in $\lfloor \text{delay} / 10 \rfloor$; `perform_refresh` resets `consecutive_failures` and `next_allowed_refresh_timestamp` on success).
    - *Documents*: `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, `spec/supporting/design.md`.
  - **AC-005 (Explicit refresh and stale-read revalidation bypass backoff; TTL separation)**:
    - *Source*: `src/cache.rs` (`is_backing_off` is only evaluated by `refresh_loop` when creating eligible future streams; `refresh()` and `schedule_revalidation()` do not check backoff).
    - *Documents*: `README.md`, `README_zh.md`, `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, `docs/wiki/en/Configuration.md`.
  - **AC-006 (Language alignment & zero runtime modifications)**:
    - *Documents*: English, Simplified Chinese, and Traditional Chinese across all READMEs and wiki pages aligned. No executable code files (`.rs`, `.js`, `.sh`) modified.
- **Tooling Verification**:
  - `node scripts/check-doc-links.mjs`: 417 links verified, 0 broken.
  - `node scripts/check-doc-examples.mjs`: 4 canonical snippets compiling cleanly under `native-tls` and `rustls`.
  - `git diff --check`: Clean, no whitespace or formatting errors.
