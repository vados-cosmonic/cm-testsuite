# Building and checking

The initial supported host is Linux x86_64. Host prerequisites are Git, Bash, curl, tar, xz, coreutils, and a native C compiler/linker/archiver. A preinstalled Rust compiler is optional.

```sh
bash scripts/bootstrap.sh
export PATH="$PWD/.tools:$PATH"
./just build
./just check
./just demo
./just site
```

The bootstrap downloads checksum-verified DotSlash 0.5.9 into `.tools/`. Repository launchers pin Just 1.59.0, Buck2's 2026-09-15 release, and mdBook 0.5.4. Buck downloads the Rust 1.98.1 host distribution using wasi-testsuite's release hashes, extracting just rustc, the native standard library, Cargo, rustfmt, and Clippy. Wasm SDKs and documentation archives are not extracted.

Buck declares source files, the toolchain, and every registry crate archive as inputs. The Cargo action builds with a fresh Cargo home, a generated directory source, `--locked --offline`, and explicit compiler paths. Only dependency-fetch actions need the network. Cargo's actual feature, build-script, and proc-macro graph is retained; there is no second manually maintained package manifest. Reindeer is not a dependency of this first draft. A later conversion to native per-crate Buck rules would need to preserve this graph.

`./just test` builds Buck's `//:tests` check target, which runs Cargo's unit, integration, and documentation tests. `./just lint` runs Clippy with warnings denied. `./just fmt-check` runs pinned rustfmt. These are cached Buck build/check actions rather than individually exposed native Buck test targets. A changed source or lockfile invalidates the action.

Updating Rust dependencies requires updating `rust/Cargo.lock`, then running `./just buck-lock`. `./just xtask buck-lock --check` checks the generated archive declarations. Added packages must first be included in those declarations; a developer can bootstrap generation with `cargo xtask buck-lock` from `rust/`. All ordinary build dependencies are checksum-pinned; the host C toolchain and OS are not yet hermetic, and byte-identical builds across machines have not been established.

For a Cargo workflow, install the version specified by `rust/rust-toolchain.toml`, enter `rust/`, and use `cargo xtask`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. The library APIs and command behavior are the same.

The mdBook is generated under `site-out/docs/`; the submitted-results page is separate at `site-out/results/`. `site-out/index.html` links to both using relative paths, supporting project Pages URLs. CI builds the site without publishing. The Pages workflow is manually dispatched and requires repository Pages configuration.
