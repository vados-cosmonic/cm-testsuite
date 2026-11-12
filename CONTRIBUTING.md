# Contributing to cm-testsuite

Contributions can improve the exchange tooling, documentation, or the tests
shared by participating projects. Open an issue to discuss a new capability or
submit a pull request that explains the problem and resulting behavior.

## Contributing tests

Keep authoritative WAST tests in your own project and enroll them through a
tracked `.cm-testsuite.toml`. Describe their purpose, required features, and
coverage so receiving projects can make informed routing decisions. The
[manifest guide](docs/src/manifest.md) documents the schema, while the
[example projects](examples/README.md) demonstrate contributing and receiving
within the same repository. Check that you have permission to redistribute the
tests and retain applicable license notices.

## Developing the tooling

Follow the [build guide](docs/src/building.md) to install DotSlash and native
prerequisites, then run these commands from the checkout root:

```sh
dotslash bins/just check
dotslash bins/just demo
dotslash bins/just site
dotslash bins/just zizmor --offline
```

Rust is downloaded by Buck. If you prefer a local Rust installation, use the
version pinned in `rust/rust-toolchain.toml` and the Cargo commands documented
in the build guide.

Keep domain behavior in the workspace libraries and CLI handling in the
corresponding `rust/xtask/src/cmd/` module. Document public functions concisely,
separate semantic responsibilities into sibling modules, and explain dense
validation or reconciliation logic. Workspace dependencies disable default
features; each consuming crate enables only the features it needs.

## Adding examples and tests

Every directory immediately under `examples/` is an E2E fixture. Include a
minimal README, a manifest that contributes at least one WAST test, and a
receiving route that accepts another example's test. The E2E flow discovers
these projects automatically and checks repeatability, byte preservation, and
protection of original files. Add focused regression coverage for behavior
changes, especially file ownership, stale plans, and metadata preservation.

## Test scenarios

Local E2E coverage lives in `rust/xtask/tests/scenarios/`, with repository and CLI
helpers in `tests/common.rs`. Run it through `dotslash bins/just test`, or use
`cargo test -p xtask --test local` from `rust/` when developing with local Cargo.
Each scenario creates isolated temporary directories and real Git history; no
network credentials are needed. Group new scenarios by behavior and reuse the
fixtures rather than copying command setup.

Fixtures used by more than one crate live in `rust/crates/test-support`
(`cm-test-support`): `github::FakeGitHub`, an in-memory GitHub repository behind
the `cm_github::Api` boundary, and `git`, which runs Git in throwaway
repositories without the developer's configuration. Depend on it only from
`[dev-dependencies]`, and extend the fake there instead of writing another mock.

The live GitHub scenario is ignored by default. See
[MAINTAINERS.md](MAINTAINERS.md#live-github-tests) for its opt-in command,
credentials, temporary repositories, and cleanup behavior.

## Updating dependencies and documentation

Keep `rust/Cargo.lock` and `.buck/third-party/crates.bzl` synchronized using the
[dependency update procedure](docs/src/building.md#updating-dependencies).
Declare dependencies through `{ workspace = true }` in leaf manifests and group
local path dependencies separately from external crates.

Write documentation for users and maintainers, describe actual behavior, and
keep operational maintainer procedures in [MAINTAINERS.md](MAINTAINERS.md).
Use GitHub-compatible admonitions for warnings and tables for pinned versions.

## Preparing a pull request

Explain the behavior changed, why it matters, and the checks you ran. Use
Conventional Commits with a concise title. Keep `.llm/` plans and working notes
local, and avoid committing generated build output or credentials.

For commit-message checks, install Node.js 24 or newer and run
`npm ci --ignore-scripts`. The [release guide](docs/src/releases.md#commit-messages)
lists the allowed Conventional Commit types and scopes and the local lint command.
