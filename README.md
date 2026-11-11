# cm-testsuite

cm-testsuite is a Rust toolkit for curating and exchanging checked-in Component
Model WAST tests. Projects retain ownership of their tests while a shared catalog
records reviewed snapshots and proposes selected updates to other projects.

cm-testsuite only gathers and redistributes tests; it does not execute them.
Engines run tests in their own harnesses and can submit results for display
alongside the catalog, without implying certification or conformance.

The `xtask` command provides acquisition, import and delivery planning, GitHub
publication, and result admission. Workspace libraries expose the same operations
for integration into other tools; see the [architecture](docs/src/architecture.md)
and [tooling design FAQ](docs/src/faq/why-not-use-tool-x.md) for details.

## Dependencies

Install [DotSlash](https://dotslash-cli.com/docs/installation/) before running
repository commands. cm-testsuite relies heavily on DotSlash to fetch and verify
pinned tool binaries, avoiding local builds or separate installations of Buck2,
Just, mdBook, and Zizmor. Their launchers live in `bins/`; Buck downloads the
pinned Rust SDK, so you do not need Rust installed to build or use the toolkit.

Linux, macOS, and Windows are supported on x86_64 and ARM64. Builds also require
Git, a native C compiler/linker and archiver, and the shell and archive utilities
listed in the [build guide](docs/src/building.md). That guide covers Xcode tools
on macOS and the Visual Studio developer environment on Windows. Your first
build needs network access to download the pinned tools and dependencies.

## Try it locally

From the checkout root, check prerequisites, build the command entrypoint, and
run the example exchange:

```sh
dotslash bins/just doctor
dotslash bins/just build
dotslash bins/just demo
```

The demo copies every project under `examples/` into `.demo/`, creates local Git
histories, and gathers their tests into a shared catalog. Each project then
receives tests from its peers. The flow checks exact delivered bytes, preserves
original source files, and verifies that repeated plans produce no changes.
It runs without GitHub credentials or network access once build inputs are cached.

Run `dotslash bins/just check` to execute the checks, or `dotslash bins/just site`
to build the documentation and results pages under `site-out/`. Developers who
already have the pinned Rust version can use the same commands through Cargo:

```sh
cd rust
cargo xtask --help
cargo test --workspace --locked
```

## Main operations

Acquire tests from a project's committed `.cm-testsuite.toml`, prepare a
reviewable import plan, and apply it to the local catalog:

```sh
dotslash bins/just xtask acquire --source github:OWNER/REPO --reference main --out source.toml
dotslash bins/just xtask plan-import --snapshot source.toml --out import.toml
dotslash bins/just xtask apply --plan import.toml
dotslash bins/just xtask validate
```

Use `--manifest` to select another tracked manifest or `--local /path/to/git/repo`
to acquire committed local files. The [workflow guide](docs/src/workflow.md)
explains receiving policies and explicit draft pull-request publication; the
[example projects](examples/README.md) show how one project can both contribute
and receive tests.

## Features

1. **Traceable snapshots:** immutable source commits, optional per-file pins,
   stable source/name identities, and SHA-256 verification preserve provenance.
2. **Reviewable changes:** deterministic TOML plans describe imports, source
   retirements, and deliveries before guarded local application.
3. **Selective receiving:** routes filter by capabilities and categories, honor
   permanent exclusions, and support managed moves and opt-in deletions.
4. **GitHub proposals:** caller-authorized draft pull requests update managed
   branches while preserving unrelated files and remembering declined plans.
5. **Structural analysis:** WAST parsing retains intentionally invalid or malformed
   payloads used in negative tests and supplies conservative syntax evidence.
6. **Submitted results:** approved publishers can submit TOML observations for
   display on a separate results page alongside the static documentation.

## Current limitations

Feature inference is incomplete, so receiving routes must explicitly accept
uncertain analysis with `allow_unknown = true`. Syntax evidence helps review
routing decisions but does not establish every runtime requirement of a test.

Acquisition supports checked-in WAST from Git repositories. Identity aliases,
automatic registry polling, general URL/S3 sources, generated tests, and forks as
delivery destinations are not currently supported. GitHub publication is limited
to 200 changed files on a branch in the destination repository.

Local application uses a lock to coordinate cm-testsuite writers and rolls back
ordinary write failures. Changes spanning multiple files are not crash-atomic,
and the lock cannot prevent another program from editing those files.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development checks, example projects,
and guidance on contributing tests or code.

Licensed under Apache-2.0; see [LICENSE](LICENSE) and
[third-party attribution](THIRD_PARTY.md).
