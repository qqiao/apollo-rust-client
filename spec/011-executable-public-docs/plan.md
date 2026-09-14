# Plan: Public documentation correctness and drift checks

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Read and recheck the inventory

Read baseline contracts, package 007's public alias and package 008's ownership changes. Inspect source enum/struct definitions rather than treating old prose as the source of truth.

Known current discrepancies (also inspect all search matches; this is a minimum inventory):

| Family | Baseline issue | Required correction |
|---|---|---|
| en/Configuration.md native example near line 326 and environment factory near line 479 | Missing public struct fields | Complete builder examples; preserve a compiling fallible factory signature |
| en/Upgrade-Guide.md near line 135 and translated copies | Recommends `use apollo_rust_client::{Client, ClientConfig}` | Use `client_config::ClientConfig`; do not add root alias |
| en/Error-Handling.md and en/Design-Overview.md | Nonexistent NamespaceNotFound and incorrect placement of DeserializeError; incomplete categories | Actual top-level and nested enums, public CacheError after 007 |
| en/Features.md and en/Configuration.md secret guidance | Automatic IP detection; per-namespace secret | IP explicitly configured; secret belongs to ClientConfig/client |
| en/Configuration.md cache directory tree | Legacy unhashed names | Current v2 digest filenames under `<base>/apollo-rust-client/config-cache`; no format migration |
| Design-Cache and Client listener docs | Claims all internal locks released | Only memory/listener-list locks released; callbacks synchronous; load/refresh coordination may remain held |
| CHANGELOG Unreleased | Automated suite no longer requires Docker | Default/all includes real Docker integration; explicit fast is Docker-independent |
| English/zh-CN design language switches | Eight missing zh-TW file targets | Remove unavailable target from those switches; retain existing alternatives |
| spec/005 status text | Implemented heading coexists with planning-session wording | Separate historical planning provenance from current status without inventing evidence |

Do not treat every `.await` found in migration guides as a defect: there are valid explicitly labeled old-version “before” blocks. Do not replace occurrences globally. Formatting complaints are a separate baseline observation.

Search checklist:

```sh
rg -n 'NamespaceNotFound|DeserializeError|Client, ClientConfig|ClientConfig \{|automatic IP|per namespace|per-namespace|internal locks|no longer requires Docker' README.md README_zh.md CHANGELOG.md docs/wiki src/lib.rs src/cache.rs
rg -n '\.to_object\(\)\.await|\.get_(string|int|float|bool)\([^)]*\)\.await' docs/wiki
```

Review search results semantically; JavaScript `import { Client, ClientConfig }` is valid, and historical imports are not necessarily current examples.

## Error taxonomy to document

Verify exact source before editing, then use:

- Public top-level Error: AlreadyRunning, Namespace, Cache, Config, HttpClient, Refresh(String).
- CacheError (007): Io, Serde, Reqwest, HttpStatus { status, body }, Timeout { seconds }, CoalescedRefresh(String), UrlParse, InvalidBaseUrl(String), InvalidSigningKey.
- namespace::Error: Json(json::Error), Yaml(yaml::Error), Text(String), Xml(String). Both json::Error and yaml::Error contain ContentNotFound and DeserializeError variants. These are valid nested variants, not direct variants of namespace::Error; use their full paths. Do not remove or rename them.
- JSON `to_object` returns serde_json::Error; YAML `to_object` returns noyalib::Error. These are synchronous consumer conversions; do not promise they are always wrapped by Client Error.
- Native persistence failures are normally best-effort/logged, not guaranteed fatal namespace errors. HTTP 404 is HttpStatus, not NamespaceNotFound. Coalesced and listener errors retain existing snapshot limitations.

Keep the `cache` module private. Link to the public alias/source rustdoc rather than suggest access to private Cache.

## Executable Rust snippet inventory

Create `scripts/check-doc-examples.mjs` with Node built-ins only. It has an explicit required inventory of these four required pairs:

1. `docs/wiki/en/Configuration.md` — marker `<!-- apollo-example: native-config -->` before a complete builder `rust` fence.
2. `docs/wiki/en/Upgrade-Guide.md` — marker `<!-- apollo-example: public-imports -->` before a current supported import/configuration `rust` fence.
3. `docs/wiki/en/Error-Handling.md` — marker `<!-- apollo-example: public-errors -->` before a current typed match `rust` fence using CacheError.
4. `docs/wiki/en/Configuration.md` — marker `<!-- apollo-example: environment-config -->` before the corrected current environment-specific factory. Give the factory a fallible Result signature if builder/env operations can fail; do not invent `ClientConfig::default()` or use a partial struct literal. The checker compiles the factory definition without calling environment lookups.

If adding translated executable snippets, register them explicitly as additional document/ID pairs. Short translated pages can instead link to the canonical code. The example inventory is not a claim every doc block is compiled.

For each required pair, require exactly one matching marker followed immediately (allow blank lines) by one closed Rust fence. Reject wrong language, duplicate IDs in that document, absent closing fence, and empty block. Do not silently scan whatever happens to be present or treat zero blocks as success. Do not execute/compile historical before blocks. Marked snippets must be valid **function bodies** requiring only apollo-rust-client and std; any helper function definitions can be local items. Do not mark async/network setup examples in this first inventory.

Generate one temporary Cargo consumer package under an OS temp directory. Manifest details:

- Unique private package name, edition matching the repository (2024), `publish = false`. Seed its temporary Cargo.lock with a copy of the repository Cargo.lock so compatible locked dependency versions are reused rather than resolving an unrelated fresh graph. Cargo may add the temporary root package only in that copy; never write the repository lockfile. If a real dependency re-resolution is required, report it rather than quietly treating a different graph as equivalent verification.
- `[dependencies.apollo-rust-client] path = <absolute repo root>, default-features = false`.
- Forward package features `native-tls = ["apollo-rust-client/native-tls"]`, `rustls = ["apollo-rust-client/rustls"]`, with default `["native-tls"]`.
- Each example becomes a named binary with `fn main() -> Result<(), Box<dyn std::error::Error>> { <extracted source>; Ok(()) }`. Use predictable generated filenames and preserve a map to original document/marker/line.
- Wrapper-only allowances for unused imports/variables/dead code are acceptable in teaching examples. Do not suppress type errors or compile the examples under `cfg(FALSE)`; do not add catch-all lint suppressions to production.

Run `cargo clippy --manifest-path <temp>/Cargo.toml --all-targets -- -D warnings` then the same command with `--no-default-features --features rustls`. This is compilation, not `cargo check` or network execution. Spawn Cargo with argument arrays, propagate status, include document/marker context on compiler failure, and clean the temporary package in finally. Keep working directory at the repository root so configured registry/source settings apply. Do not force offline mode unless the caller environment requests it: first use may need existing dependency downloads. Do not use `--all-features`.

For reasonable repeated cost, use a dedicated build-artifact subdirectory under the repository's ignored target path via CARGO_TARGET_DIR; this is compiler output, never hand-edited source. The temp manifest/lock/source remain outside the checkout and no dependency/version change is committed. Do not run this checker recursively from itself or invoke `scripts/test.sh` within it.

Negative checks: copy the doc/fixture into a temp input fixture or expose pure extraction logic to a Node test; prove absent/duplicate marker fails. Temporarily modify a selected snippet only in an isolated test copy to reference an unavailable symbol and prove the compiler invocation fails. Avoid a checker “test” that merely greps its own implementation. Do not add permanent invalid snippets to current docs.

## Local file-link checker

Create `scripts/check-doc-links.mjs`, Node built-ins only. Scope: root README.md, README_zh.md, CHANGELOG.md; all tracked/current Markdown below docs/wiki and spec. Do not traverse .git, target, pkg, skill catalogs, or ignored directories. Relative links resolve from the containing document; URL-decode path components, strip fragments, and handle angle-bracket-wrapped paths where present. Ignore http/https/mailto and fragment-only links. Heading anchors are out of scope; report this honestly.

Handle inline and reference-style Markdown file links used by these docs, excluding fenced code examples of link syntax. Treat a local file link to a missing target as nonzero failure listing file/line/target. An explicit new-doc fixture with a broken path should prove the checker fails. Do not ignore whole languages/directories to hide current failures; do not follow or contact external URLs.

The eight known broken destinations are zh-TW/Design-{Overview,WASM,ClientConfig,Client}.md, linked once each from corresponding en and zh-CN pages. Correct the source switches rather than invent empty translated documents.

## Integration and documentation batches

Add both Node checker commands to the fast path in scripts/test.sh and scripts/apollo-test.sh, once each, preserving prior 007/009/010 work and propagating failures. Package 008's generated WASM ownership smoke stays in build verification; do not substitute a Rust compile check for JS execution.

Use small language/family batches in tasks.md. Update shared contracts/traceability only for actually corrected/current examples and planned checker scope. Fix the known erroneous listener lock-release claim in source rustdoc without changing executable code. Correct planning-status wording in spec/005 while preserving dated historical claims as history rather than fabricating new approvals or CI results.

Verify checkers directly, then `scripts/test.sh fast`. No new Docker run is necessary for pure doc/checker changes; the final program checkpoint runs full integration once. Link tests/compiler checks are the meaningful tests here, not new tests for whitespace or prose wording. Package 012 separately owns polling text; avoid overlapping edits until this package lands.
