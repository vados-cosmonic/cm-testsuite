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
| `dotslash bins/just build` | Compile the shared CLI and test artifacts |
| `dotslash bins/just test` | Run unit, integration, and documentation tests |
| `dotslash bins/just lint` | Run Clippy with warnings denied |
| `dotslash bins/just fmt-check` | Check Rust formatting |
| `dotslash bins/just check` | Run formatting, Clippy, compilation, and Buck test suites |
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

`.buck/toolchains/rust/releases.bzl` also records WASI Preview 1 and Preview 2 standard
libraries. The `wasm32-wasip3` entry remains commented out because the pinned
stable release does not publish it; enable it only with an official checksum
when upgrading to a stable release that includes the target. Native exchange
commands do not download these Wasm standard libraries.
Cargo retains its feature, build-script, and procedural-macro dependency graph.

Compilation and lint checks are cached Buck actions. Changing a declared source
or lockfile invalidates the relevant action. The `//:artifacts` target compiles the
workspace once and publishes a relocatable bundle containing xtask, unit and
integration harnesses, and the libraries and sources needed by rustdoc.
`//:xtask` selects its executable from that bundle.

`buck2 test //:tests` runs the `//:unit-tests`, `//:integration-tests`, and
`//:doc-tests` suites. These are test executions, not cached build steps: a repeat
invocation reuses compilation but executes the suites again. Cargo supplies the
exact rustdoc arguments during compilation; rustdoc builds and runs documentation
examples only when the documentation suite executes. The live GitHub suite is
excluded and remains behind `just test-github` and its explicit opt-in.
Harnesses run from their owning package directory in the bundle, with relocated
`CARGO_MANIFEST_DIR` and `CARGO_MANIFEST_PATH` values. All offline suites, including
documentation tests, remove `GH_TOKEN` and `GITHUB_TOKEN` from child environments.

Cargo retains responsibility for individual crate compilation. Buck owns the
workspace artifact and test-suite graph; this does not yet provide per-crate
incremental actions or remote execution.

The host C toolchain and operating system remain external prerequisites.
The build is not fully hermetic, and byte-identical outputs across machines
are not guaranteed.

## Continuous integration

The `check` workflow first selects its runners from the paths a pull request changes.
A pull request confined to `registry/`, `metadata/`, `corpus/`, `reports/`, `docs/`, `.agents/`,
`.github/ISSUE_TEMPLATE/`, root-level Markdown, and `LICENSE` runs one `ubuntu-latest` job.
Buck reads none of those paths, so that job skips formatting, Clippy, the build, and the test suites.
Any other path, and every push to `main`, merge-queue run, and manual run, selects all six runners.

| Step | Six-runner change | Data or documentation change |
| --- | --- | --- |
| `just fmt-check` | `ubuntu-latest` | not run |
| `just lint` | `ubuntu-latest`, `windows-latest` | not run |
| `just ci-build`, `just ci-test` | every runner | not run |
| `just demo` | every runner | `ubuntu-latest` |
| `xtask buck-lock --check`, `xtask validate`, release-tool checks, `just site` | `ubuntu-latest` | `ubuntu-latest` |

rustfmt, the lockfile check, validation, the release-tool checks, and the site give the same answer on every host.
Clippy differs only where `cfg` selects different code,
and the workspace conditions code on `unix` and `windows` alone, so one runner of each family lints.
Add a runner's `lint` flag in `ci.yml` before introducing code conditional on another operating system or architecture.
The final `ci` job fails unless every selected job succeeded.

`just ci-build` writes Buck's build report to `.ci-results/build.json`, and
`just ci-test` executes all offline suites with separate build and command reports.
The demo, release-tool checks, and site generation reuse the compiled xtask in the same job.
CI uploads each runner's reports and one `site-preview` artifact.

The reusable `.github/actions/setup-buck2` action installs Just and DotSlash,
restores DotSlash's platform-specific download cache, and verifies the repository's
Buck2 launcher. Cache keys include the operating system, architecture, and launcher
contents. It deliberately uses the repository pin: the [installer linked by Buck2](https://github.com/dtolnay/install-buck2)
currently downloads a floating release and selects x86_64 on Windows.

Linux and macOS jobs also use commit-pinned [Rebuck](https://github.com/gilescope/rebuck)
to run a loopback-only bazel-remote cache and persist its contents through Actions cache.
The cache has a 4 GiB disk limit per runner variant.
The `//.buck/platforms:ci-cache` execution platform keeps compilation local while allowing
Buck to download and upload action results through REAPI.
The setup action writes `.buckconfig.local` before starting Buck;
local development continues to use the default execution platform.
Windows keeps the shared build/test flow without this persistent action cache,
because Rebuck does not support Windows yet.

Cache namespaces include the matrix runner and Buck launcher pin.
The setup action also records the hosted runner's OS, architecture, image name,
and image version in the environment of SDK and Cargo actions.
That environment participates in Buck's action keys, preventing reuse across host
toolchain updates even when an older cache archive is restored.
If CI changes the host compiler or SDK after provisioning the runner image,
extend this identity to describe that change before enabling reuse.

Repeated runs can reuse SDK extraction, formatting, Clippy, and the compiled bundle.
Every test suite still executes, as do the demo, release checks, and site generation.
Caching is at workspace-action granularity: changing a Rust source rebuilds the
bundle and reruns Clippy; it does not provide per-crate reuse across source edits.
Compare build-step timings and Buck's cache-hit summaries on a fresh runner rerun
to measure the benefit, including cache download and upload time.
The existing build and test reports remain available as workflow artifacts.

## Updating dependencies

After updating `rust/Cargo.lock`, run `dotslash bins/just buck-lock` to regenerate
`.buck/third-party/crates.bzl`.
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
