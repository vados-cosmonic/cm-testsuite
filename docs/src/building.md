# Building and checking

## Prerequisites

The supported build host is Linux x86_64.
Other hosts fail the prerequisite check before Buck starts.

Install [DotSlash](https://dotslash-cli.com/docs/installation/) and Bash,
Git, a native C compiler/linker and archiver, tar, xz, and coreutils.
These tools must be available on your normal system PATH.
Rust does not need to be installed.

Use an installed Just or the repository's `./just` launcher.
The latter requires DotSlash and selects the pinned Just version.
No repository-specific PATH changes or bootstrap downloads are needed.

```sh
./just doctor
./just build
./just check
./just demo
./just site
```

`doctor` checks the host and required system commands without starting a build.
Build recipes run it automatically and report missing tools before invoking Buck.
`just zizmor` checks only its launcher prerequisites; it does not require a C compiler.

## Pinned tools

The launchers and build definitions select the following tool versions.
DotSlash verifies launcher downloads, and Buck verifies Rust and crate inputs.

| Tool | Pin | Used by |
| --- | --- | --- |
| Just | 1.59.0 | Repository launcher and CI |
| Buck2 | 2026-09-15 | Build and check targets |
| Rust | 1.98.1 | Native compiler, standard library, Cargo, rustfmt, and Clippy |
| mdBook | 0.5.4 | Documentation builds |
| Zizmor | 1.32.0 | Local and CI workflow audits |
| Wasmono | `5cb97a58be3853916a0fcb7f27eeca2524d58ea5` | Buck build utilities |
| DotSlash | 0.5.7 | CI; installed separately for local use |
| cargo-binstall | 1.27.0 | CI tool installation on cache misses |

The shared `.github/actions/setup-tools` action caches Just and DotSlash binaries
by operating system, architecture, and version.
On a cache miss, cargo-binstall prefers prebuilt binaries and falls back to Cargo
installation when necessary.
All related workflows use this action and invoke `just` directly.

## Build and check commands

| Command | Purpose |
| --- | --- |
| `./just doctor` | Check host prerequisites without building |
| `./just build` | Build the Rust task entrypoint |
| `./just test` | Run unit, integration, and documentation tests |
| `./just lint` | Run Clippy with warnings denied |
| `./just fmt-check` | Check Rust formatting |
| `./just check` | Run formatting, tests, and Clippy |
| `./just demo` | Exercise local acquisition, import, and receiving |
| `./just site` | Generate documentation and submitted-result pages |
| `./just zizmor` | Audit GitHub Actions |

Set `GH_TOKEN` for Zizmor's online audits, or use `./just zizmor --offline`
for checks that do not need GitHub access.
CI runs the same auditor version with its read-only GitHub token.
Workflows default to `permissions: {}` and grant permissions per job.

## Build isolation

Buck declares workspace sources, the Rust SDK, and registry crate archives as inputs.
It extracts only the native Rust components needed for builds and checks;
Wasm SDKs and Rust documentation archives are not extracted.

Each Cargo action uses a fresh Cargo home, a generated directory source,
`--locked --offline`, and explicit compiler paths.
Only dependency-fetch actions need network access.
Cargo retains its feature, build-script, and procedural-macro dependency graph.

Builds and checks are cached Buck actions.
Changing a declared source or lockfile invalidates the action.
Buck wraps the Cargo workspace rather than exposing individual crates or tests
as native Buck targets.

The host C toolchain and operating system remain external prerequisites.
The build is not fully hermetic, and byte-identical outputs across machines
are not guaranteed.

## Updating dependencies

After updating `rust/Cargo.lock`, run `./just buck-lock` to regenerate
`third-party/crates.bzl`.
Use `./just xtask buck-lock --check` to verify that they agree.

New packages must appear in the Buck declarations before Buck can build xtask.
To regenerate declarations when adding a package, use `cargo xtask buck-lock`
from `rust/` with the Rust version in `rust/rust-toolchain.toml` installed.

## Using Cargo directly

Install the Rust version specified in `rust/rust-toolchain.toml` and enter `rust/`.
The library APIs and command behavior are shared with the Buck build.

```sh
cargo xtask --help
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Generated documentation and results

The site entrypoint is `site-out/index.html`.
It links to the mdBook under `site-out/docs/` and submitted engine results under
`site-out/results/` using relative paths, including on project Pages URLs.

Pull-request CI builds the site without publishing it.
The `publish-pages` workflow publishes on every push to `main` and also supports
manual dispatch; a newer run cancels an older run still in progress.
Repository Pages configuration is required before deployment can succeed.
