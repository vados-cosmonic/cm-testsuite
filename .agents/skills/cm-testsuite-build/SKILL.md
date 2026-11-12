---
name: cm-testsuite-build
description: Build, test, lint, or format cm-testsuite through Buck2 and DotSlash, diagnose Buck or launcher failures, change Buck definitions under .buck/, or update Rust dependencies and tool pins.
---

# Building and checking

Commands start at the checkout root.
The [build guide](../../../docs/src/building.md) is the reference for prerequisites, pinned versions, and build isolation.

## Running checks

- DotSlash is the only tool to install.
  `dotslash bins/just doctor` reports missing host prerequisites without building.
- The first build downloads Buck2, the Rust SDK, and about 230 crate archives, and needs network access.
  Budget several minutes and a few gigabytes of free disk;
  `No space left on device` in a Cargo action is a host problem, not a code failure.
- Use the narrowest recipe that covers the change:

  | Recipe | Checks |
  | --- | --- |
  | `dotslash bins/just fmt-check` | rustfmt; needs no crates and finishes quickly |
  | `dotslash bins/just lint` | Clippy across every target, warnings denied |
  | `dotslash bins/just test` | Unit, integration, and documentation suites |
  | `dotslash bins/just check` | All of the above plus the build |
  | `dotslash bins/just demo` | The example exchange end to end, offline |
  | `dotslash bins/just site` | mdBook and the results site |

- CI runs more than `check`.
  Reproduce the `check (ubuntu-latest)` job by running the steps in [ci.yml](../../../.github/workflows/ci.yml) in order.
  The other runners skip the steps whose result cannot depend on the host;
  see [Continuous integration](../../../docs/src/building.md#continuous-integration).
- Test suites execute on every invocation; only compilation is cached.
- With a local Rust matching `rust/rust-toolchain.toml`, Cargo works from `rust/`
  and is faster for iteration, for example `cargo test -p xtask --test local`.
  Buck remains the check that CI runs, so finish with the Just recipes.

## Nested checkouts

Buck2 uses the outermost ancestor directory containing `.buckconfig` as the project root,
unless it finds a `.buckroot` file first.
The empty `.buckroot` at the repository root keeps Buck inside a checkout nested in another one,
such as a Git worktree under `.claude/worktrees/`.
Without it, Buck builds the outer checkout and reports success for code you did not change.

- Do not delete or move `.buckroot`.
- If results do not reflect your changes, run `dotslash bins/buck2 root --kind project`.
  It must print the checkout you are working in.
- A `File changed: root//<path to your checkout>/...` line in Buck's output means Buck is rooted in a parent directory.

## Buck layout

- `.buckconfig`, `.buckroot`, and the root `BUCK` must stay at the repository root.
- `.buck/` holds everything else that only Buck reads:

  | Path | Contents |
  | --- | --- |
  | `.buck/build-support/` | The `cargo_workspace` rules and their Cargo wrapper script |
  | `.buck/platforms/` | The `ci-cache` execution platform used only by CI |
  | `.buck/third-party/` | Generated crate archive targets and checksums |
  | `.buck/toolchains/` | The `toolchains` cell: Rust SDK download and extraction |

- Refer to root-cell packages as `//.buck/<package>:<target>` and the toolchains cell as `toolchains//...`.
- Sources reach Cargo through the globs in the root `BUCK`:
  the workspace manifest and lockfile, `Cargo.toml` and `*.rs` files under `rust/crates/` and `rust/xtask/`,
  and everything under `examples/`.
  Any other file, such as a data file read with `include_str!`, is invisible to the Buck build until a glob covers it,
  even though local Cargo finds it.

## Dependencies and pins

- `.buck/third-party/crates.bzl` is generated from `rust/Cargo.lock`. Never edit it.
  After changing the lockfile, regenerate it with `dotslash bins/just buck-lock`
  and verify with `dotslash bins/just xtask buck-lock --check`.
- A newly added crate must be declared before Buck can build `xtask`, so the Just recipe cannot bootstrap it.
  Run `cargo xtask buck-lock` from `rust/` with local Cargo instead.
- Declare dependencies in `[workspace.dependencies]` with an exact version and `default-features = false`.
  Leaf manifests use `{ workspace = true }` and enable only the features they need.
- When changing the Rust version or a launcher in `bins/`, verify the published checksums
  and update the [pinned-tools table](../../../docs/src/building.md#pinned-tools).
