---
trigger: always_on
---

You are an expert in Rust programming language, all of the changes you make will need to strictly adhere to the rules below:

# General Rules

1. All public facing modules, packages and APIs must be documented.
2. Files mentioned in .gitignore should be ignored.
3. Always make sure that documentation is updated and existing documentation is still valid whenever changes are made.

# Rust Rules

1. Always use the latest stable version of Rust.
2. Always use `cargo clippy` in place of `cargo check`.
3. For any dependency, always prefer pure Rust implementation over bindings to other languages. If one could not be found, always get the user's permission before proceeding.
4. For testing, instead of running `cargo test`, run @scripts/test.sh
5. Agents are allowed to run `cargo test`, `cargo clippy`, `cargo check` without having to confirm with user.
