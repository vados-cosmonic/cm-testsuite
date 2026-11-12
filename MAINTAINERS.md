# Maintainer workflows

## Reviewing exchange proposals

Review the source manifest, provenance, WAST changes, and receiving policy before
applying or merging a proposal. The import and receiving workflows are manually
invoked and publish draft pull requests with [Octo STS](https://github.com/octo-sts/app) tokens.
Consult the [workflow guide](docs/src/workflow.md) for local equivalents and token
requirements; running a build or test does not publish a proposal.

## Running the scheduled exchange

Projects ask to join through the "Enroll a repository" issue form.
Check that the manifest is committed and, for a receiving repository,
that `.github/chainguard/cm-testsuite.sts.yaml` is on its default branch;
a `propose-receiving-update` run for that repository with `publish` enabled confirms the policy works,
and needs no enrollment.
Enroll a repository by adding it to `registry/participants.toml` through review,
with the GitHub logins of the people who can fix it under `maintainers`.
List this repository's own maintainers in the top-level `maintainers` array.
The [workflow guide](docs/src/workflow.md#gather-and-distribute) documents the format
and the `gather` and `distribute` commands, which run locally exactly as they do in CI.

No exchange workflow uses a stored token.
Each one publishes with a short-lived Octo STS token scoped to a single repository,
granted by a trust policy committed in that repository:
`.github/chainguard/import.sts.yaml` here for the import pull request,
and `.github/chainguard/cm-testsuite.sts.yaml` in every receiving repository.
Adding a receiving repository, or a workflow that publishes, means editing those policies;
see [Credentials in GitHub Actions](docs/src/workflow.md#credentials-in-github-actions).

Both workflows can be dispatched manually with `only` or `except` to limit a run
to some of the enrolled repositories.

A repository that fails does not fail the scheduled run.
Each workflow keeps one open issue here that lists the failing repositories,
mentions their maintainers and this repository's, and closes itself after a clean run.
Fix the cause, or remove the repository from the registry to stop trying.
To stop a workflow altogether, disable it in the repository's Actions settings.

Gathered imports arrive as one pull request on the `cm-testsuite/gather` branch, opened ready for review.
A run cannot change the draft state of one that is already open; mark it ready by hand.
Closing it without merging declines that exact import until the sources change.
To get it back, run `gather-updates` by hand with `publish` and `recreate` checked.
Review and merge it like any other import;
`distribute-updates` then offers the merged changes to receiving repositories.

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

Every job that builds with Buck calls `.github/actions/setup-buck2`,
which restores and serves the [Buck action cache](docs/src/building.md#action-cache).
Only the `check` job on the default branch saves that cache.
Pass `action-cache: "false"` in a job that publishes what it builds,
or that holds write access while running in a pull request's cache scope.

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
scope (plus `delete_repo` for automatic cleanup); fine-grained credentials need permission to create
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

To retain the repositories for inspection and delete them yourself afterward, set
`CM_TESTSUITE_KEEP_GITHUB_REPOS=1` as well:

```sh
CM_TESTSUITE_LIVE_GITHUB=1 CM_TESTSUITE_KEEP_GITHUB_REPOS=1 dotslash bins/just test-github
```

This mode prints each retained repository URL on success or failure and never
deletes repositories. It does not require the classic token's `delete_repo` scope.
All three repositories share the same `cm-testsuite-e2e-<run-suffix>-` prefix,
followed by `source`, `catalog`, or `recipient`.

## Releases

See the [release guide](docs/src/releases.md) for release preparation, exact-commit
tagging, GitHub artifacts, and post-release notifications.
