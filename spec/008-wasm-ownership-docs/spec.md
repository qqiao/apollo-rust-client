# Feature Specification: Correct WASM ownership guidance

**Feature ID:** `008-wasm-ownership-docs`
**Created:** 2026-09-13
**Status:** Implemented and verified in baseline remediation (see tasks.md).
**Review:** R3 / P2. Baseline: [004](../004-javascript-client/spec.md), 004/FR-010 and 004/AC-012; [ownership contract](../supporting/contracts.md#javascript-consumer-boundary).

## Purpose and scope

JavaScript developers must be able to follow the documentation without freeing consumed configuration wrappers, leaking still-owned wrappers, or replacing an application error with a cleanup error.

This package corrects and executes documentation for the **existing** by-value constructor. It does not change WASM binding signatures, make Client borrow/clone ClientConfig, add idempotent-free wrappers to generated code, or promise cancellation of detached work.

## Stories and acceptance scenarios

### Story 1 — Clean up only owned objects (P1)

As a JavaScript developer, I want to know which objects I own after construction and retrieval.

**Independent test:** Build a real Node/WASM package and execute the canonical documented ownership block with no network dependency.

- **AC-001:** Given a live ClientConfig transferred to `new Client(config)`, when the documented operation finishes, then cleanup frees the live Client once and does not reuse/free the consumed config; the sequence throws no cleanup error.
- **AC-002:** Given an untransferred ClientConfig, when it is no longer needed, then its documented cleanup frees it once.
- **AC-003:** Given an application error after successful Client construction, when cleanup runs, then the original error remains observable and is not replaced by a consumed-config free error.
- **AC-004:** Given a valid ClientConfig wrapper containing invalid settings, when Client construction consumes it and rejects validation, then the error is observable and cleanup does not attempt to free that consumed wrapper. Documentation must distinguish this from failure before transfer is attempted.

### Story 2 — Use namespace values with correct lifetimes (P2)

As a JavaScript developer, I want to release wrappers without freeing ordinary JavaScript data.

**Independent test:** Run generated-package format tests and inspect their finally blocks.

- **AC-005:** Given a returned Properties wrapper, when its use ends on success or error, then it is freed once. Given JSON/YAML/Text or listener data, then no WASM-wrapper cleanup is instructed for those ordinary JS values.
- **AC-006:** Given equivalent English, Simplified Chinese, or Traditional Chinese current examples, when their guidance is followed, then they describe the same ownership rules and do not recommend freeing config after transfer.

## Functional requirements

- **FR-001:** Current docs MUST distinguish unconsumed ClientConfig from config consumed by Client construction.
- **FR-002:** Current examples MUST NOT call `.free()` or access fields on consumed config; cleanup MUST act only on live, owned wrappers.
- **FR-003:** Exception-safe examples MUST preserve application/construction failures while releasing still-owned resources.
- **FR-004:** Docs MUST distinguish Client/Properties wrappers from ordinary JSON/YAML/Text/listener values and MUST NOT present Cache as an exported WASM class.
- **FR-005:** At least one canonical ownership snippet from documentation MUST be executed against generated bindings; tests MUST cover successful construction, untransferred config, application failure, and constructor-validation failure.
- **FR-006:** The fix MUST preserve the existing public constructor and getter/lifecycle signatures and real-network integration isolation.

## Entities and success criteria

**Entities:** untransferred config, consumed wrapper, live Client, owned Properties, ordinary JS value, original exception.

- **SC-001:** AC-001–004 execute successfully against generated bindings and require no string-matching of a specific wasm-bindgen null-pointer message.
- **SC-002:** Current ownership instructions across both READMEs and all wiki languages satisfy AC-005/006; real Node/WASM format tests still pass.
- **SC-003:** Existing method-export smoke coverage remains intact and catches missing exports; no generated file is patched by hand.

## Assumptions

The by-value transfer occurs when the generated constructor receives a valid wrapper, before Rust validation. Verify this with the installed generated package when implementing. Arbitrary invalid JS argument types are not specified by AC-004. A documentation class may provide its own idempotent cleanup only if it clears its own references after freeing; this is not a change to `.free()` itself. See [plan](plan.md) and [tasks](tasks.md).
