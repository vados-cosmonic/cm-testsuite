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

Supply `GITHUB_TOKEN` through the environment or a credential manager, then explicitly run:

```sh
dotslash bins/just xtask publish --repository github:OWNER/TARGET --plan /tmp/receive.toml
```

The token's origin is up to the caller; no GitHub App is required. Reading private sources needs repository read access. Publishing needs contents and pull-request write permissions for the target. GitHub's workflow event behavior also depends on the token used; a repository `GITHUB_TOKEN` is not a general cross-repository credential. Tokens are not included in plans or logs.

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

Both commands only report unless told otherwise, and accept `--only github:OWNER/REPO`
to narrow a run.
Reading public repositories needs no token;
`--publish` needs `GITHUB_TOKEN` with contents and pull-request write on each destination.

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
and keeps one issue per command in that repository instead:
the issue lists each failing repository, its error, and its maintainers,
is rewritten in place by later failing runs,
and is closed by the next clean run.
Reporting uses `REPORT_TOKEN` when set and `GITHUB_TOKEN` otherwise;
the token needs issue write access.

The `gather-updates` and `distribute-updates` workflows run these commands daily.
`distribute-updates` also runs when the catalog or the registry changes on `main`.
Scheduled runs publish with the `CM_TESTSUITE_TOKEN` secret and report failures as issues;
without the secret they only report planned changes.
Manually dispatched runs publish only when their `publish` input is enabled.

## Editable records

Catalogs, acquisition snapshots, plans, receiving receipts, publisher registries,
and submitted reports use TOML. Programmatic updates preserve comments on
surviving fields and tables; deleting a field also removes its attached comments.
Canonical record digests depend on values, while plan guards hash exact file
bytes so any edit after review requires replanning.

The record editor uses `toml` and `toml_edit` from the maintained
[toml-rs project](https://github.com/toml-rs/toml), also used by Cargo.
GitHub API payloads and DotSlash launcher manifests retain their required JSON format.
