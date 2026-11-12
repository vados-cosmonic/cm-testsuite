---
name: cm-testsuite-rust
description: Implement, refactor, or review Rust code in the cm-testsuite workspace under rust/, including library crates, xtask subcommands, shared test fixtures, and local end-to-end scenarios.
---

# Rust changes

Read [the architecture guide](../../../docs/src/architecture.md) before moving code between crates.

## Where code belongs

- Domain behavior lives in the `cm-*` library crates under `rust/crates/`.
  Libraries take explicit inputs and return structured values and errors.
  They must not depend on `xtask` or on GitHub Actions.
- `xtask` owns argument parsing, configuration, credentials, progress, and exit reporting.
  Add a subcommand as a module under `rust/xtask/src/cmd/` that calls into a library.
- Reach GitHub only through the `cm_github::Api` boundary so that tests can inject a fake.
- Before adding a helper, search the owning crate, `rust/xtask/src/io.rs`, and `cm-test-support`.

## Style

- Document public functions concisely.
- Split semantic responsibilities into sibling modules rather than growing one file.
- Explain dense validation or reconciliation logic in a comment placed before it.
- Default to private items, then `pub(crate)`; make an item `pub` only when another crate needs it.
- The workspace uses edition 2024 with default rustfmt and Clippy settings.
  Fix warnings rather than allowing them; `lint` denies them.

## Invariants to preserve

These are the behaviors the exchange exists to guarantee.
Add a regression test when a change touches one.

- Application checks the entire plan before mutating anything.
- Every removal or move checks the managed baseline digest first.
- Original source files and files the tool does not manage are never modified.
- Repeating a plan after applying it produces no changes.
- A fetch error is an error, never an empty source; only a successful snapshot can retire tests.
- Filesystem symlinks and Git symlink or submodule entries are rejected.
- WAST bytes are preserved exactly, including line endings.

## Tests

- Local end-to-end scenarios live in `rust/xtask/tests/scenarios/`, with helpers in `rust/xtask/tests/common.rs`.
  Group a new scenario with related behavior and reuse the fixtures instead of copying command setup.
- Scenarios create isolated temporary directories and real Git history.
  They must not need network access or credentials; offline suites strip `GH_TOKEN` and `GITHUB_TOKEN`.
- Fixtures shared by more than one crate live in `rust/crates/test-support` (`cm-test-support`):
  `github::FakeGitHub` and the isolated `git` helpers.
  Depend on it only from `[dev-dependencies]`, and extend the fake instead of writing another mock.
- `rust/xtask/tests/github_live.rs` is ignored by default and creates real private repositories.
  Compile it, but run it only when explicitly asked;
  see [MAINTAINERS.md](../../../MAINTAINERS.md#live-github-tests).

## Before committing

Follow [Building and checking](../cm-testsuite-build/SKILL.md): run `dotslash bins/just check`,
and regenerate the Buck crate inputs when `rust/Cargo.lock` changed.
