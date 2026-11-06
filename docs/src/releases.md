# Repository hygiene and releases

cm-testsuite uses the Conventional Commit and release workflow structure from
[bytecodealliance/jco](https://github.com/bytecodealliance/jco/tree/0851dc9bc8bfa0313e16358a8cd937ccc407d7a3).
The release scripts are Rust library functions exposed through `xtask`; Node.js
is needed only for commitlint.

## Commit messages

Use `type(scope): description`, for example:

```text
feat(catalog): record source retirement
fix(delivery): preserve excluded tests
ci(release): validate the tagged workspace
release: cm-testsuite v0.2.0
```

Scopes are optional. Allowed scopes include the Rust crate names without the
`cm-` prefix (`acquisition`, `catalog`, `delivery`, `exchange-plan`, `github`,
`results`, `wast-analysis`, `release`), `xtask`, `corpus`, `registry`, `build`,
`toolchains`, and general scopes listed in `commitlint.config.mjs`.
The configuration also lists the allowed types, including `sec` and `release`.

Install Node.js 24 or newer and the locked development dependencies:

```sh
npm ci --ignore-scripts
./just xtask lint-commits --from origin/main --to HEAD
printf '%s\n' 'fix(catalog): preserve identity' | npm exec --no -- commitlint
```

The hygiene workflow checks every proposed commit and the PR title (which may
become a squash commit), including merge-queue ranges. Existing history outside
the proposed range is not re-linted. It also audits the workflows with zizmor. Dependabot groups weekly GitHub Actions and commitlint updates.
Branch protection can require `commits`, `workflows`, and the existing `check`
job; installing these workflows does not change repository protection settings.

## Release lifecycle

The entire Rust workspace shares one version and one tag,
`cm-testsuite-vVERSION`. Release artifacts currently target Linux x86_64 and
include the `xtask` binary, license/readme files, and SHA-256 checksums. Source
archives are available through GitHub Releases. The crates remain
`publish = false`; these workflows do not publish to crates.io or npm.

1. Run **create-release-pr** manually with a SemVer version greater than the
   current workspace version, without a leading `v`. Prereleases such as
   `0.2.0-rc.1` are supported. The workflow reads `main`, updates
   `rust/Cargo.toml`, every workspace entry in `rust/Cargo.lock`, and
   `CHANGELOG.md`, runs checks, and opens a signed-off release preparation PR.
2. Review the changelog and merge the release PR into `main`. Preserve its
   generated title, branch name, and `release-pr` label. The workflow creates
   the label automatically when preparing a release.
3. **tag-release** validates the merged PR's repository, branch, label, version,
   and exact merge commit. It refuses to tag a commit outside `origin/main`,
   mismatched workspace/lockfile/changelog versions, or move an existing tag.
4. **release** checks out the exact tag, refuses tags outside `origin/main`
   (including maintainer-pushed tags), runs checks, packages the Linux binary,
   uploads assets to a draft, publishes the GitHub release, then updates related
   issues or PRs. Published assets are never overwritten by reruns. Prereleases
   are marked as such. Releases do not change GitHub's Latest designation
   automatically, so rerunning an older release cannot promote it over a newer
   one; maintainers can select Latest in GitHub after publication.

The tag workflow explicitly calls the reusable release workflow. It does not
rely on `GITHUB_TOKEN` pushes triggering another workflow. Tags pushed by a
maintainer can also trigger the release workflow directly.

The default `GITHUB_TOKEN` is sufficient for tagging, release publication, and
notifications. No token is passed to the release build job; publishing and
notification permissions are scoped to their own jobs.

**create-release-pr** opens the release PR with a short-lived
[Octo-STS](https://github.com/octo-sts/app) token rather than `GITHUB_TOKEN`.
PRs opened with `GITHUB_TOKEN` start no `pull_request` workflows,
so required checks would never run on them.
To set this up:

- Install the [Octo-STS GitHub App](https://github.com/apps/octo-sts/installations/new)
  on this repository.
- Keep `.github/chainguard/create-release-pr.sts.yaml` in sync with the repository.
  It trusts only `create-release-pr.yml` dispatched on `main`,
  and grants `contents`, `pull_requests`, and `issues` write.
  Its subject uses the repository's immutable owner and repository IDs;
  a fork or transferred copy needs its own subject from the repository's
  **Settings → Actions → OIDC** page.

The workflow mints the token only after the release checks have run.

For recovery, rerun **tag-release** with a version and a ref reachable from main,
or **release** with an existing full tag. Tagging is idempotent when the tag
already points at the validated commit. A failed notification job can be rerun
without publishing again. Do not dispatch any of these workflows before the
release tooling has landed on `main`.

## Local preparation and inspection

Use a clean disposable checkout with full history and tags. Bootstrap the
repository tools first; the checksum-pinned `git-cliff` launcher generates
changelogs offline from Git history. Stable release notes include changes from
intervening prereleases; prerelease notes start after the previous release.

```sh
./just xtask release metadata --version 0.2.0-rc.1
./just xtask release prepare --version 0.2.0-rc.1
./just xtask release extract-latest-changelog-section --version 0.2.0-rc.1 --out /tmp/notes.md
./just xtask release semver-is-prerelease 0.2.0-rc.1
./just xtask release semver-get-prerelease 0.2.0-rc.1
./just xtask release semver-lt 0.2.0-rc.1 0.2.0
```

`release from-event` validates a merged-PR event JSON file. `release validate`
checks the current checkout against an existing release tag. Metadata commands
can append validated values to `--github-output PATH` for GitHub Actions.

## Post-release issue and PR updates

```sh
./just xtask release post-release-issue-update \
  --repository OWNER/REPO --tag cm-testsuite-v0.2.0
# Inspect the printed plan, then explicitly send the comments:
./just xtask release post-release-issue-update \
  --repository OWNER/REPO --tag cm-testsuite-v0.2.0 --apply
```

The command reads the published release and commits in the exact tagged range,
finds merged PRs whose merge commits belong to that release's history, and
notifies issues referenced by closing keywords in their bodies. Recognized
forms include `Fixes #123`, `Closes OWNER/REPO#123`, and
`Resolves https://github.com/OWNER/REPO/issues/123`. Cross-repository references,
code fences, and inline code are ignored. A PR without recognized issue
references receives the notification itself. Issues are not closed or reopened.
A linked reference that is a PR is described as a PR.

Some references are skipped with a warning instead of failing the run:

- references that do not exist (HTTP 404), such as typos
- issues that were transferred (HTTP 301) or deleted (HTTP 410)

The first release has no previous tag,
so its range would be the repository's entire history;
it sends no notifications.

Pagination and duplicate references are handled. Only marker comments authored
by `github-actions[bot]` are updated; for a caller-owned token, set
`--comment-author YOUR_LOGIN` to the token's actual comment author. Existing
release links are retained, identical reruns do not write, and an older rerun
cannot replace a newer release link. The workflow serializes notifications
across releases. If a comment changes after planning, the command fails and can
be rerun safely. `--apply` requires `GITHUB_TOKEN`; without it the command only
prints the proposed edits. Tests use an in-memory API and do not post comments.
