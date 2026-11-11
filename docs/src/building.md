# Building and checking

## Prerequisites

The pinned SDK and launchers support these hosts:

| Operating system | Architectures | Native prerequisites |
| --- | --- | --- |
| Linux (glibc) | x86_64, ARM64 | Bash, Git, C compiler/linker, archiver, tar, gzip, xz, coreutils |
| macOS | Intel, Apple Silicon | zsh, Bash, Xcode Command Line Tools, Git, tar, gzip, xz |
| Windows | x86_64, ARM64 | Git Bash with tar/gzip/xz/coreutils, Visual Studio C++ Build Tools and Windows SDK |

Install [DotSlash](https://dotslash-cli.com/docs/installation/) separately.
System tools must be available on your normal PATH; Rust need not be installed.
On macOS, install the Command Line Tools with `xcode-select --install` and xz
through your package manager. Just runs recipes with zsh on macOS and Bash on
Linux and Windows; the small SDK/build wrappers use Bash on every host.

On Windows, launch Git Bash from a Visual Studio Developer Command Prompt for
your architecture so `cl.exe`, `lib.exe`, and the Windows SDK are available.
Enable Windows Developer Mode (or run with symlink privileges), because Buck
uses symlinks for declared inputs. Just discovers Git Bash and forwards the
native compiler environment into Buck actions; no repository-specific PATH
changes are needed. WSL is also supported through the Linux path.
The Windows ARM64 documentation and audit launchers use upstream x86_64 binaries
under Windows emulation; the Rust SDK, Buck, and Just run natively.

Use an installed Just or the repository's `dotslash bins/just` launcher.
The latter requires DotSlash and selects the pinned Just version.
No repository-specific PATH changes or bootstrap downloads are needed.
`scripts/bootstrap.sh` invokes the same prerequisite check through pinned Just.

```sh
dotslash bins/just doctor
dotslash bins/just build
dotslash bins/just check
dotslash bins/just demo
dotslash bins/just site
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
| git-cliff | 2.14.2 | Release changelog generation |
| Zizmor | 1.32.0 | Local and CI workflow audits |
| Wasmono | `5cb97a58be3853916a0fcb7f27eeca2524d58ea5` | Buck build utilities |
| DotSlash | 0.5.7 | CI; installed separately for local use |
| cargo-binstall | 1.27.0 | CI tool installation on cache misses |

## Build and check commands

| Command | Purpose |
| --- | --- |
| `dotslash bins/just doctor` | Check host prerequisites without building |
| `dotslash bins/just build` | Build the Rust task entrypoint |
| `dotslash bins/just test` | Run unit, integration, and documentation tests |
| `dotslash bins/just lint` | Run Clippy with warnings denied |
| `dotslash bins/just fmt-check` | Check Rust formatting |
| `dotslash bins/just check` | Run formatting, tests, and Clippy |
| `dotslash bins/just demo` | Exercise local acquisition, import, and receiving |
| `dotslash bins/just site` | Generate documentation and submitted-result pages |
| `dotslash bins/just zizmor` | Audit GitHub Actions |

Set `GH_TOKEN` for Zizmor's online audits, or use `dotslash bins/just zizmor --offline`
for checks that do not need GitHub access.

## Build isolation

Buck declares workspace sources, the Rust SDK, and registry crate archives as inputs.
It extracts only the native Rust components needed for builds and checks;
Wasm SDKs and Rust documentation archives are not extracted.

Each Cargo action uses a fresh Cargo home, a generated directory source,
`--locked --offline`, and explicit compiler paths.
Only dependency-fetch actions need network access. Buck downloads crate archives
with checksum verification, and the Cargo wrapper extracts them directly into
its directory source. Formatting uses the SDK and sources without fetching crates.

`toolchains/rust/releases.bzl` also records WASI Preview 1 and Preview 2 standard
libraries. The `wasm32-wasip3` entry remains commented out because the pinned
stable release does not publish it; enable it only with an official checksum
when upgrading to a stable release that includes the target. Native exchange
commands do not download these Wasm standard libraries.
Cargo retains its feature, build-script, and procedural-macro dependency graph.

Builds and checks are cached Buck actions.
Changing a declared source or lockfile invalidates the action.
Buck wraps the Cargo workspace rather than exposing individual crates or tests
as native Buck targets.

The host C toolchain and operating system remain external prerequisites.
The build is not fully hermetic, and byte-identical outputs across machines
are not guaranteed.

## Updating dependencies

After updating `rust/Cargo.lock`, run `dotslash bins/just buck-lock` to regenerate
`third-party/crates.bzl`.
Use `dotslash bins/just xtask buck-lock --check` to verify that they agree.

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
