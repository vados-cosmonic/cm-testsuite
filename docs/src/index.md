# cm-testsuite

cm-testsuite curates copies of checked-in WAST tests from opted-in repositories. Each test keeps its authoritative source. Consumers decide how to execute it; this project aggregates and structurally checks scripts without running them.

The toolkit provides reusable Rust library APIs and a single xtask command entrypoint. All exchange operations can run locally. GitHub Actions wraps those same commands and supplies credentials only for explicitly requested publication.

Start with `dotslash bins/just demo` after following the [build instructions](building.md). The demo uses temporary local Git histories and verifies idempotent collection and receiving. It does not contact GitHub or execute Wasm.
