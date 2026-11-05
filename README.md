# cm-testsuite

A Rust toolkit for curating and exchanging checked-in Component Model WAST tests. Sources retain ownership; this repository reviews exact-byte snapshots and proposes selected updates to consumers. It never executes WAST.

This is an initial implementation draft. The workspace contains reusable libraries for acquisition, catalog/provenance, structural analysis, change planning, delivery, GitHub access, and engine-submitted results. `xtask` is the sole command entrypoint. See [architecture](docs/src/architecture.md) and [why we did not adopt the evaluated tools](docs/src/faq/why-not-use-tool-x.md).

## Try it locally

The validated host is Linux x86_64. Install Git, a C compiler/linker and archiver, Bash, curl, tar, xz, and coreutils. Rust does **not** need to be installed for the Buck path.

```sh
bash scripts/bootstrap.sh
export PATH="$PWD/.tools:$PATH"
./just build
./just demo
./just check
./just site
```

The demo creates two disposable Git repositories under `.demo/`, imports their committed WAST, and delivers it into a third directory. It verifies that repeating both plans produces no changes. No network or credentials are used by the demo itself; cold tool/dependency downloads require network access.

Buck downloads checksum-pinned Rust 1.98.1 and uses Cargo's locked dependency graph with declared crate archives and offline build actions. Buck, Just, DotSlash, mdBook, and Wasmono are pinned. The compiler's host C toolchain remains a prerequisite. See [build details](docs/src/building.md).

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

Acquisition requires a tracked `.cm-testsuite.toml`. `--manifest` overrides its location; `--local /path/to/git/repo` reads committed local objects for offline work. Receiving and explicit draft-PR publication are documented in [the workflow guide](docs/src/workflow.md). There is no automatic publication as part of a build or test.

## What this draft handles

- Immutable source commits, optional per-file pins and integrity checks, stable source/name IDs, and per-file SHA-256.
- Explicit WAST paths, source retirement records, deterministic plans, and guarded local application.
- Multiple receiving routes, capability/category filters, permanent exclusions, managed moves, and opt-in deletion proposals.
- Caller-token GitHub draft PR creation/update, identical-plan decline suppression, and preservation of unrelated files. GitHub mutation behavior is covered by an in-memory API model, not a live consumer-repository pilot.
- Structural WAST parsing that retains deliberately invalid/malformed test payloads.
- Approved-publisher JSON result admission and separate static documentation/results pages.

## Current limits

Feature inference is intentionally incomplete; routes must explicitly opt into uncertain analysis with `allow_unknown = true`. Syntax evidence is not a complete set of runtime requirements. The draft does not implement identity aliases, automatic registry polling, forks as delivery destinations, general URL/S3 sources, or generated tests. Publication is limited to 200 changed files and same-repository branches. Local application coordinates this tool with a lock and rolls back ordinary write errors; it is not crash-atomic across multiple files and cannot lock out unrelated editors.

No real sources or result publishers are enrolled by default. CI checks changes without secrets; exchange and Pages publication are manual workflows. Planning material under `.llm/` is ignored and must never be committed.

Licensed under Apache-2.0. See [third-party attribution](THIRD_PARTY.md).
