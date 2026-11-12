# Collection and receiving workflow

All commands below are explicit local actions. `plan-import` and `plan-receive` produce reviewable TOML; `apply` changes files. Builds and tests never open PRs.

## Import

```sh
dotslash bins/just xtask acquire --source github:OWNER/REPO --reference main --out /tmp/source.toml
dotslash bins/just xtask plan-import --snapshot /tmp/source.toml --out /tmp/import.toml
dotslash bins/just xtask apply --plan /tmp/import.toml
dotslash bins/just xtask validate
```

Repeat `--snapshot` to combine sources in one plan. A source snapshot is complete or fails; a failed download cannot retire tests. Each test can specify its own immutable revision. The import records exact bytes under `corpus/wast/OWNER/REPO/NAME.wast` and metadata under `metadata/catalog.toml`.

For local fixtures, add `--local /path/to/source/git/repo` to acquisition. `--manifest config/custom.toml` selects a nondefault manifest. Local Git reads committed blobs only. Neither backend invokes a producer's build process.

Source removals create explicit catalog retirement records. They do not automatically delete received files. Changes to a test's declared name are treated as a new identity: do not rename identities until the exclusion/migration implications have been reviewed.

## Receive

Check out the receiving repository's trusted default branch and prepare its opt-in manifest. From the cm-testsuite checkout:

```sh
dotslash bins/just xtask --root /path/to/recipient plan-receive \
  --catalog-root "$PWD" --recipient github:OWNER/RECIPIENT \
  --out /tmp/receive.toml
dotslash bins/just xtask --root /path/to/recipient apply --plan /tmp/receive.toml
```

Plans include a `.cm-testsuite-lock.toml` receipt and exact suggested exclusion IDs. Adding an exclusion preserves an already received file. Changing a route can move a managed file after baseline checks. An unrelated file at the destination is a collision even if the tool has not previously managed that path.

Application preflights policy/catalog and file digests, then stages replacement content.
Repeating a fully applied plan is a no-op.
It coordinates other instances with `.cm-testsuite-write-lock`, rechecks writes,
and attempts rollback on ordinary I/O errors.

> [!WARNING]
> Application is not a transaction across process crashes or arbitrary concurrent editors.
> Inspect the repository before removing a stale lock.
> Use a clean disposable worktree for automation.

## Publish a reviewed proposal

Supply `GITHUB_TOKEN` or `GH_TOKEN` through the environment, or sign in with `gh auth login`, then explicitly run:

```sh
dotslash bins/just xtask publish --repository github:OWNER/TARGET --plan /tmp/receive.toml
```

The token's origin is up to the caller; no GitHub App is required. Reading private sources needs repository read access. Publishing needs contents and pull-request write permissions for the target. GitHub's workflow event behavior also depends on the token used; a repository `GITHUB_TOKEN` is not a general cross-repository credential. Tokens are not included in plans or logs. Inside GitHub Actions, `--sts-identity NAME` publishes with an [Octo STS](#credentials-in-github-actions) token instead.

Publication uses the target's default branch, verifies plan guards against its committed state, and creates or updates one stable **draft** PR per destination repository: every import shares one branch in the catalog repository, and each receiving repository has its own. A later plan replaces the contents of that PR. It preserves unrelated files through the base tree. It does not auto-merge. A declined identical plan is suppressed; permanent exclusions remain the durable way to decline future versions. When the proposed changes disappear, publication closes the owned PR after checking its branch, retaining the branch for inspection. Unexpected branch edits and non-fast-forward races fail without force-pushing.

Automatic withdrawals are marked separately from maintainer declines, so a withdrawn proposal can be offered again if it becomes relevant. Empty plans must still match their policy/catalog guards before withdrawing a PR. Retrying an already-applied plan can close only that exact proposal, leaving newer proposals for the same subject open.

Publication supports branches in the destination repository and caps proposals at 200 file edits. Manual exchange workflows prepare artifacts first and publish only when their publication input is enabled.

## Gather and distribute

`gather` and `distribute` run the import and receiving flows for every repository
enrolled in `registry/participants.toml`:

```toml
schema = 1
maintainers = ["LOGIN"] # mentioned on every failure report

[[participant]]
repository = "github:OWNER/REPO"
manifest = ".cm-testsuite.toml" # optional
reference = "main"              # optional; gathered from the default branch when absent
maintainers = ["LOGIN"]         # optional; mentioned when this repository fails
```

Enrollment only makes the exchange look at a repository.
Its own manifest decides whether it shares tests, receives them, or both.

```sh
dotslash bins/just gather                 # report what the catalog would import
dotslash bins/just gather --apply         # write the import into this checkout
dotslash bins/just gather --publish --repository github:OWNER/CATALOG

dotslash bins/just distribute             # report what each repository would receive
dotslash bins/just distribute --publish   # open, update, or withdraw one draft PR each
```

Both commands only report unless told otherwise.
Narrow a run with `--only OWNER/REPO,OWNER/REPO` or `--except OWNER/REPO`;
each flag takes a comma-separated list or repeats, the two cannot be combined, and a name that is not enrolled is an error.
Reading public repositories needs no token.
`--publish` needs contents and pull-request write on each destination.
A local run takes the token from `GITHUB_TOKEN`, then `GH_TOKEN`,
then the GitHub CLI's login (`gh auth token`).
A GitHub Actions run passes `--sts-identity NAME` to use [Octo STS](#credentials-in-github-actions) instead;
outside GitHub Actions the flag is ignored, so a workflow's command line also runs locally.

- `gather` acquires every enrolled repository that shares tests
  and plans one import covering all of them.
  A repository that cannot be read is left out,
  so it neither blocks the others nor retires its own tests.
- `distribute` plans against each repository's default branch through the GitHub API;
  no checkout of the recipient is needed.
  A repository whose manifest does not enable receiving is skipped.
- Each destination has one draft pull request.
  A repeated run updates that pull request in place,
  and withdraws it when nothing is left to propose.
- Pull request descriptions give the change counts,
  one row per test with its source revision and summary,
  and collapsed sections for declining tests and for warnings.

A failing repository makes the command exit non-zero after the others have been processed.
With `--report-to github:OWNER/REPO` the command exits cleanly
and keeps one issue per command in that repository instead.
The option applies to whole-registry runs only, so it cannot be combined with `--only` or `--except`:
a partial run would close an issue that still describes repositories it skipped.
The issue lists each failing repository, its error, and its maintainers,
is rewritten in place by later failing runs,
and is closed by the next clean run.
Reporting uses `REPORT_TOKEN` when set and the publishing credentials otherwise;
the token needs issue write access.

The `gather-updates` and `distribute-updates` workflows run these commands daily.
`distribute-updates` also runs when the catalog or the registry changes on `main`.
Unattended runs publish and report failures as issues.
Manually dispatched runs take `only` and `except` inputs,
each a comma-separated list of `OWNER/REPO` names,
publish only when their `publish` input is enabled,
and fail instead of filing an issue.

## Credentials in GitHub Actions

No workflow uses a stored token.
Reads and the failure issue use the job's own `GITHUB_TOKEN`,
which covers this repository and public ones;
private participants are out of reach.
Writes use a short-lived [Octo STS](https://github.com/octo-sts/app) token
that the command requests for one repository, just before its first write there.
A token is granted only when that repository has the Octo STS app installed
and commits a trust policy naming the calling workflow.

| Destination | Policy file in the destination | Identity | Trusted workflows |
| --- | --- | --- | --- |
| This repository (import PR) | `.github/chainguard/import.sts.yaml` | `import` | `gather.yml`, `import.yml` |
| Each receiving repository | `.github/chainguard/cm-testsuite.sts.yaml` | `cm-testsuite` | `distribute.yml`, `receive.yml` |

A receiving repository installs the
[Octo STS app](https://github.com/apps/octo-sts/installations/new) and adds:

```yaml
issuer: https://token.actions.githubusercontent.com
subject: repo:vados-cosmonic@123968127/cm-testsuite@1447064737:ref:refs/heads/main
claim_pattern:
  job_workflow_ref: vados-cosmonic/cm-testsuite/\.github/workflows/(distribute|receive)\.yml@refs/heads/main

permissions:
  contents: write
  pull_requests: write
```

The subject carries this repository's immutable owner and repository IDs;
a fork or transferred copy substitutes its own,
from the repository's **Settings → Actions → OIDC** page.

Widening the exchange is therefore a configuration change on both sides:

- A new receiving repository is enrolled in `registry/participants.toml` here
  and commits the policy above there.
  Until it does, `distribute` reports that repository as failing and carries on with the rest.
- A new source needs only enrollment, since gathering reads and writes nothing in the source.
- A new workflow that publishes is added to the `job_workflow_ref` pattern
  of every policy it should be able to use.

The command revokes each token as soon as it has finished with that repository,
before moving on to the next.
A failed revocation is a warning, and the token then expires on its own within the hour.

## Editable records

Catalogs, acquisition snapshots, plans, receiving receipts, publisher registries,
and submitted reports use TOML. Programmatic updates preserve comments on
surviving fields and tables; deleting a field also removes its attached comments.
Canonical record digests depend on values, while plan guards hash exact file
bytes so any edit after review requires replanning.

The record editor uses `toml` and `toml_edit` from the maintained
[toml-rs project](https://github.com/toml-rs/toml), also used by Cargo.
GitHub API payloads and DotSlash launcher manifests retain their required JSON format.
