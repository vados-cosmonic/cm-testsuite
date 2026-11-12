# 📋 Component Model Test Suite

`cm-testsuite` exchanges checked-in Component Model WAST tests between projects.
Source projects keep ownership of their tests;
this repository holds a catalog of reviewed, digest-pinned snapshots
and proposes selected tests to receiving projects as draft pull requests.

`cm-testsuite` does not run tests.
Engines run them in their own harnesses
and may [submit results](docs/src/results.md) for display, without implying conformance.

## How to use

A repository can submit tests, receive them, or do both.
Both roles are configured by a committed `.cm-testsuite.toml` at your repository root.

See the [manifest reference](docs/src/manifest.md) and the [example projects](examples/README.md).

### Submit tests

Declare each test in your manifest and commit it with the WAST files:

```toml
schema = 1

[[test]]
name = "resources/borrow-after-drop" # identity: github:OWNER/REPO/<name>
path = "file:///tests/resources/borrow-after-drop.wast"
summary = "Reject using a borrow after its owner is dropped"
requires = ["component-model", "resources"]
covers = ["resource-lifetime"]
```

Then open a "Propose a hand-written WAST test" issue naming your repository.
A maintainer imports it, locally or through the `propose-source-import` workflow:

```sh
dotslash bins/just xtask acquire --source github:OWNER/REPO --reference main --out source.toml
dotslash bins/just xtask plan-import --snapshot source.toml --out import.toml
dotslash bins/just xtask apply --plan import.toml
dotslash bins/just xtask validate
```

- `file:///` paths are relative to the repository root,
  `file://` to the manifest.
- Acquisition reads committed blobs at one immutable commit.
  It never runs your build.
- Import copies exact bytes to `corpus/wast/OWNER/REPO/NAME.wast`
  and records the source commit and SHA-256 in `metadata/catalog.toml`.
- `name` is the identity.
  Keep it when moving or editing a file;
  a new name is a new test.
- Removing a test retires it in the catalog.
  Receivers keep their copy unless they opt in to deletions.
- Fix a test in your repository and re-import.
  Snapshots here are not edited.

### Receive tests

Opt in and describe where tests go:

```toml
schema = 1

[receive]
enabled = true
additions = "pull-request" # default
updates = "pull-request"   # default
deletions = "manual"       # default
exclude = []               # identities to decline permanently

[[receive.route]]
name = "resources"
destination = "file:///tests/shared/resources"
supports = ["component-model", "resources"] # must include every `requires` of a test
covers_any = ["resource-lifetime"]          # empty accepts any coverage
allow_unknown = true                        # see below
```

Plan from a cm-testsuite checkout against a checkout of your default branch,
then apply locally or publish a draft pull request:

```sh
dotslash bins/just xtask --root /path/to/recipient plan-receive \
  --catalog-root "$PWD" --recipient github:OWNER/RECIPIENT --out receive.toml

dotslash bins/just xtask --root /path/to/recipient apply --plan receive.toml
# or, with GITHUB_TOKEN set (contents and pull-request write):
dotslash bins/just xtask publish --repository github:OWNER/RECIPIENT --plan receive.toml
```

Maintainers can run the same flow through the `propose-receiving-update` workflow.

- A test is delivered when exactly one route supports its `requires` and matches its coverage.
  Matching several routes is an error.
- Feature inference is always marked incomplete,
  so a route receives tests only with `allow_unknown = true`.
  Selection uses metadata;
  it does not show your engine passes.
- Files land at `<destination>/OWNER/REPO/NAME.wast`.
  `.cm-testsuite-lock.toml` records what the tool manages.
- Plans are TOML, guarded by file digests.
  `apply` checks the whole plan before writing,
  refuses to overwrite files it does not manage,
  and is a no-op when repeated.
  Any edit after planning requires a new plan.
- `publish` maintains one draft PR per plan subject.
  It never merges or force-pushes,
  and does not re-offer an identical plan you closed.

The [workflow guide](docs/src/workflow.md) covers each step in detail.

## Build

Install [DotSlash](https://dotslash-cli.com/docs/installation/).
It fetches the pinned Buck2, Just, mdBook, and Zizmor launchers in `bins/`,
and Buck downloads the Rust SDK.

You also need Git and a native C toolchain;
see the [build guide](docs/src/building.md) for per-platform prerequisites.
Linux, macOS, and Windows are supported on x86_64 and ARM64.

```sh
dotslash bins/just doctor # check prerequisites
dotslash bins/just build  # compile xtask
dotslash bins/just demo   # run the example exchange offline, in .demo/
dotslash bins/just check  # format, lint, build, and test
dotslash bins/just site   # docs and results pages, in site-out/
```

`xtask` is the only executable.
The workspace libraries expose the same operations;
see the [architecture](docs/src/architecture.md)
and the [tooling design FAQ](docs/src/faq/why-not-use-tool-x.md).

## Limitations

- Only checked-in WAST from Git repositories is supported:
  no generated tests, URL or S3 sources, registry polling, or identity aliases.
- Syntax evidence aids routing review
  but does not establish a test's runtime requirements.
- Publication targets branches in the destination repository, not forks,
  and is capped at 200 changed files.
- Local application locks out other cm-testsuite writers
  and rolls back ordinary write failures.
  It is not crash-atomic and cannot stop other programs editing the files.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development checks and guidance on contributing tests or code,
and [MAINTAINERS.md](MAINTAINERS.md) for maintainer procedures.

Licensed under Apache-2.0;
see [LICENSE](LICENSE) and [third-party attribution](THIRD_PARTY.md).
