# Collection and receiving workflow

All commands below are explicit local actions. `plan-import` and `plan-receive` produce reviewable JSON; `apply` changes files. Builds and tests never open PRs.

## Import

```sh
./just xtask acquire --source github:OWNER/REPO --reference main --out /tmp/source.json
./just xtask plan-import --snapshot /tmp/source.json --out /tmp/import.json
./just xtask apply --plan /tmp/import.json
./just xtask validate
```

Repeat `--snapshot` to combine sources in one plan. A source snapshot is complete or fails; a failed download cannot retire tests. Each test can specify its own immutable revision. The import records exact bytes under `corpus/wast/OWNER/REPO/NAME.wast` and metadata under `metadata/catalog.json`.

For local fixtures, add `--local /path/to/source/git/repo` to acquisition. `--manifest config/custom.toml` selects a nondefault manifest. Local Git reads committed blobs only. Neither backend invokes a producer's build process.

Source removals create explicit catalog retirement records. They do not automatically delete received files. Changes to a test's declared name are treated as a new identity in this draft: do not rename identities until the exclusion/migration implications have been reviewed.

## Receive

Check out the receiving repository's trusted default branch and prepare its opt-in manifest. From the cm-testsuite checkout:

```sh
./just xtask --root /path/to/recipient plan-receive \
  --catalog-root "$PWD" --recipient github:OWNER/RECIPIENT \
  --out /tmp/receive.json
./just xtask --root /path/to/recipient apply --plan /tmp/receive.json
```

Plans include a `.cm-testsuite-lock.json` receipt and exact suggested exclusion IDs. Adding an exclusion preserves an already received file. Changing a route can move a managed file after baseline checks. An unrelated file at the destination is a collision even if the tool has not previously managed that path.

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
./just xtask publish --repository github:OWNER/TARGET --plan /tmp/receive.json
```

The token's origin is up to the caller; no GitHub App is required. Reading private sources needs repository read access. Publishing needs contents and pull-request write permissions for the target. GitHub's workflow event behavior also depends on the token used; a repository `GITHUB_TOKEN` is not a general cross-repository credential. Tokens are not included in plans or logs.

Publication uses the target's default branch, verifies plan guards against its committed state, and creates or updates one stable **draft** PR branch for the plan subject. It preserves unrelated files through the base tree. It does not auto-merge. A declined identical plan is suppressed; permanent exclusions remain the durable way to decline future versions. When the proposed changes disappear, publication closes the owned PR after checking its branch, retaining the branch for inspection. Unexpected branch edits and non-fast-forward races fail without force-pushing.

Automatic withdrawals are marked separately from maintainer declines, so a withdrawn proposal can be offered again if it becomes relevant. Empty plans must still match their policy/catalog guards before withdrawing a PR. Retrying an already-applied plan can close only that exact proposal, leaving newer proposals for the same subject open.

The first draft supports same-repository branches, not forks, and caps proposals at 200 file edits. Its API behavior is tested with an in-memory GitHub model. A live consumer-repository integration pilot remains to be done. Manual exchange workflows prepare artifacts first and publish only when their boolean input is enabled. No source or recipient is enrolled automatically.
