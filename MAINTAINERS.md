# Maintainer workflows

## Reviewing exchange proposals

Review the source manifest, provenance, WAST changes, and receiving policy before
applying or merging a proposal. The import and receiving workflows are manually
invoked and publish draft pull requests using caller-authorized credentials.
Consult the [workflow guide](docs/src/workflow.md) for local equivalents and token
requirements; running a build or test does not publish a proposal.

## Admitting results

Approve publisher identifiers through review in `registry/publishers.toml`.
Review submitted report provenance and use the admission command described in
[the results guide](docs/src/results.md#submitting-a-report) before merging reports.
Publisher approval is an admission policy, not proof of the identity of a file's
submitter.

## Maintaining CI and tool pins

Workflows start with empty permissions and grant each job only its required
access. Use Zizmor to audit changes to workflows and shared actions. Keep tool
installation and caching in `.github/actions/setup-tools` so improvements apply
to every consumer, and update cache keys when tool pins change.

When changing Rust or a DotSlash launcher, verify the published checksums and
update the [pinned-tools table](docs/src/building.md#pinned-tools). Exercise the
host matrix and example exchange before merging platform changes.

## Publishing documentation

Configure the repository's Pages source as GitHub Actions. The `publish-pages`
workflow builds and deploys on pushes to `main`, and also supports manual runs.
Its concurrency group cancels older in-progress runs so the newest deployment
can finish. Pull requests build a site preview without deploying it.
