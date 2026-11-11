# cm-testsuite

A Rust toolkit for curating and exchanging checked-in Component Model WAST tests.
Sources retain ownership; this repository stores reviewed, exact-byte snapshots
and proposes selected updates to consumers.
It never executes WAST.

Use `xtask` for acquisition, import and delivery planning, GitHub publication,
and engine-result submissions.
The workspace libraries expose these operations for integration into other tools.
See the [architecture](docs/src/architecture.md) and
[tooling design FAQ](docs/src/faq/why-not-use-tool-x.md).

## Try it locally

The supported build host is Linux x86_64.
Install [DotSlash](https://dotslash-cli.com/docs/installation/), Git,
a C compiler/linker and archiver, Bash, tar, xz, and coreutils.
Rust does **not** need to be installed for the Buck path.

```sh
./just doctor
./just build
./just demo
./just check
./just site
```

The demo creates two disposable Git repositories under `.demo/`, imports their committed WAST, and delivers it into a third directory.
It verifies that repeating both plans produces no changes.
No network or credentials are used by the demo itself; cold tool/dependency downloads require network access.

Buck downloads checksum-pinned Rust 1.98.1 and uses Cargo's locked dependency graph with declared crate archives and offline build actions.
Buck, Just, mdBook, Zizmor, and Wasmono are pinned.
DotSlash is a system prerequisite.
The compiler's host C toolchain remains a prerequisite.
See [build details](docs/src/building.md).

Developers with Rust installed can also run the same entrypoint:

```sh
cd rust
cargo xtask --help
cargo test --workspace --locked
```

## Main operations

```sh
# Read an upstream opt-in manifest at an immutable commit; no destination writes.
./just xtask acquire --source github:OWNER/REPO --reference main --out /tmp/source.json

# Prepare a reviewable import, then apply it locally.
./just xtask plan-import --snapshot /tmp/source.json --out /tmp/import.json
./just xtask apply --plan /tmp/import.json
./just xtask validate
```

Acquisition requires a tracked `.cm-testsuite.toml`.
`--manifest` overrides its location; `--local /path/to/git/repo` reads committed local objects for offline work.
Receiving and explicit draft-PR publication are documented in [the workflow guide](docs/src/workflow.md).
There is no automatic publication as part of a build or test.

## Capabilities

- Immutable source commits, optional per-file pins and integrity checks, stable source/name IDs, and per-file SHA-256.
- Explicit WAST paths, source retirement records, deterministic plans, and guarded local application.
- Multiple receiving routes, capability/category filters, permanent exclusions, managed moves, and opt-in deletion proposals.
- Caller-token GitHub draft PR creation/update, identical-plan decline suppression, and preservation of unrelated files.
- Structural WAST parsing that retains deliberately invalid/malformed test payloads.
- Approved-publisher JSON result admission and separate static documentation/results pages.

## Current limits

Feature inference is intentionally incomplete; routes must explicitly opt into uncertain analysis with `allow_unknown = true`.
Syntax evidence is not a complete set of runtime requirements.
Unsupported operations include identity aliases, automatic registry polling,
forks as delivery destinations, general URL/S3 sources, and generated tests.
Publication is limited to 200 changed files and same-repository branches.
Local application coordinates this tool with a lock and rolls back ordinary write errors; it is not crash-atomic across multiple files and cannot lock out unrelated editors.

CI checks changes without secrets.
Run the exchange workflows manually when you want to publish proposals.
Pages publishes documentation and results on pushes to `main`;
manual publication is also available.

Licensed under Apache-2.0. See [third-party attribution](THIRD_PARTY.md).
