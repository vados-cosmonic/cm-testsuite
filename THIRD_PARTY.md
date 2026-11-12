# Third-party code and build inputs

`.buck/toolchains/rust/releases.bzl` and the downloaded-toolchain approach are adapted from [WebAssembly/wasi-testsuite](https://github.com/WebAssembly/wasi-testsuite/tree/de7fbe267210b52f63893270403177239b4f4a9b/toolchains/rust), under Apache-2.0. Its original toolchain definitions attribute an adaptation from [rue-language/rue](https://github.com/rue-language/rue). Our definitions extract only the native components and expose Cargo's actual locked graph through Buck. The Buck DotSlash launcher was also taken from that wasi-testsuite revision.

Wasmono is consumed as an external cell at `5cb97a58be3853916a0fcb7f27eeca2524d58ea5`, under Apache-2.0. We currently reuse its host-platform helpers. Its Wasm compilation and execution rules are not enabled.

Buck2, bazel-remote, bb-storage, DotSlash, Just, mdBook, Rust, and Cargo retain their upstream licenses. DotSlash descriptors and Rust release metadata record distribution hashes. `rust/Cargo.lock` records dependency versions/checksums; `.buck/third-party/crates.bzl` is generated from it. Dependencies retain the licenses supplied in their crate archives.

Tooling licenses do not grant permission to redistribute an upstream test corpus. Review each source's license and preserve its notices before admitting its tests.

The Conventional Commit configuration, release workflow structure, changelog template, and Rust ports of release metadata, SemVer/changelog helpers, and post-release issue/PR notifications are adapted from [bytecodealliance/jco](https://github.com/bytecodealliance/jco/tree/0851dc9bc8bfa0313e16358a8cd937ccc407d7a3), under Apache-2.0. The adaptations use a single Rust workspace release, exact-commit tagging, and idempotent notification planning. git-cliff and commitlint retain their upstream licenses; their distribution or package integrity is pinned.
