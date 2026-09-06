## T05 — Add real native compatibility cases

- [x] **T05 complete**

**Purpose**: Test the shipped native public API against Apollo using both native TLS configurations.

**Requirements/scenarios**: FR-001, FR-005, FR-008, FR-010; AC-007–009, AC-014.

**Dependencies**: T04.

**Files (3)**: `tests/apollo_integration.rs`, `tests/apollo/support/mod.rs`, `scripts/apollo-test.sh`.

**Work**: Add the planned real formats/identity, access-key, and grayscale cases. Use public Client/ClientConfig APIs; create separate temporary cache guards per auth variant. Gate this test target off WASM compilation, mark real cases ignored by ordinary fast test invocation, and make the orchestrator select them explicitly. Assert actual values, selected identity, types, real unauthorized error status, and gray controls. Check selected test names/count before execution. Do not import the library's private mock fixtures.

**Acceptance**:

- [x] Both native runtime feature configurations pass all format/identity/auth/grayscale cases, with no cached-success shortcut in negative cases.
- [x] Required missing environment or zero selected tests fails; ordinary fast mode still runs without a server.
- [x] Integration tests compile/lint for applicable targets without new production API exposure or unreviewed dependencies.

**Verification**: For each of `native` and `rustls`, run `scripts/test.sh integration --suite <suite> --filter <case>` for `real_apollo_formats_and_identity`, `real_apollo_access_key`, and `real_apollo_grayscale`. These are explicitly partial development runs. An unmatched filter must fail; an unfiltered native run must report the missing T06 cases until T06 is implemented. Run the three existing Clippy commands via `scripts/test.sh fast`.