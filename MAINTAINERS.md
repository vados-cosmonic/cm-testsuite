# Maintainer workflows

## Reviewing exchange proposals

Review the source manifest, provenance, WAST changes, and receiving policy before
applying or merging a proposal. The import and receiving workflows are manually
invoked and publish draft pull requests using caller-authorized credentials.
Consult the [workflow guide](docs/src/workflow.md) for local equivalents and token
requirements; running a build or test does not publish a proposal.

## Admitting results

Approve publisher identifiers through review in `registry/publishers.toml`.
Review submitted report provenance before merging reports. To run admission
checks against the current catalog and publisher allowlist, then rebuild the site:

```sh
dotslash bins/just xtask results-import --report /path/to/report.toml
dotslash bins/just site
```

See [the results guide](docs/src/results.md) for the report format.
Publisher approval is an admission policy, not proof of the identity of a file's
submitter.

## Maintaining CI and tool pins

Workflows start with empty permissions and grant each job only its required
access. Use Zizmor to audit changes to workflows and shared actions. Keep tool
installation and caching in `.github/actions/setup-tools` so improvements apply
to every consumer, and update cache keys when tool pins change. The action caches
Just and DotSlash by operating system, architecture, and version. On a cache miss,
cargo-binstall installs upstream prebuilt binaries, selecting DotSlash's release
filename for each host. Workflows invoke the installed `just` directly.

CI runs Zizmor with its read-only GitHub token. For a local audit, run
`dotslash bins/just zizmor --offline`, or set `GH_TOKEN` to include online checks.

When changing Rust or a DotSlash launcher, verify the published checksums and
update the [pinned-tools table](docs/src/building.md#pinned-tools). Exercise the
host matrix and example exchange before merging platform changes.

## Publishing documentation

Configure the repository's Pages source as GitHub Actions. The `publish-pages`
workflow builds and deploys on pushes to `main`, and also supports manual runs.
Its concurrency group cancels older in-progress runs so the newest deployment
can finish. Pull requests build a site preview without deploying it.

## Live GitHub tests

The live scenario exercises real acquisition and publication through Octocrab.
It creates private source, catalog, and recipient repositories under the
GitHub CLI's authenticated user, then publishes and merges draft import and
receiving PRs. It checks repeat publication, source updates, retirement, exact
file bytes, and preservation of unrelated files. It never uses this repository
or a caller-selected repository as a publication destination.

Install `gh` and authenticate to github.com. The test accepts `GH_TOKEN`, then
`GITHUB_TOKEN`, or the token from `gh auth token`. A classic token needs `repo`
and `delete_repo` scopes; fine-grained credentials need permission to create
private repositories and administer their contents, pull requests, and deletion.
The test checks classic scopes before creating anything. Ordinary `just test`,
`just check`, and CI compile this scenario but leave it ignored.

Run explicitly from the checkout root; Buck downloads Rust and compiles the
harness before running it outside the cached build action:

```sh
CM_TESTSUITE_LIVE_GITHUB=1 dotslash bins/just test-github
```

With local Cargo, run from `rust/`:

```sh
CM_TESTSUITE_LIVE_GITHUB=1 cargo test -p xtask --test github_live -- --ignored --nocapture
```

Temporary names begin with `cm-testsuite-e2e-` and include a unique run suffix.
Cleanup checks each repository's recorded creation ID before deleting it and
also runs on ordinary test failures. Failed cleanup is reported with exact
repository names. If the process is forcibly terminated or access is revoked,
remove those temporary repositories manually after confirming the run has ended.
