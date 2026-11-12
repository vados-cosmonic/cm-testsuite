# Warnings

Warnings never fail a command and never block a pull request.
Each kind has a stable identifier, shown wherever the warning is reported:

- Pull request descriptions group warnings by kind in a collapsed section,
  with the meaning and the action once per kind, followed by the tests or routes concerned.
- `xtask analyze` prints the analysis record on standard output
  and explains each of its warnings on standard error.
- `xtask plan-import` and `xtask plan-receive` print one line per kind on standard error.

Snapshots, plans, and `metadata/catalog.toml` store each warning as a fixed sentence.
Digests cover those sentences, so the stored text of a kind does not change;
the guidance below is attached when a warning is displayed.

| Kind | Reported by | Fires | Who acts |
| --- | --- | --- | --- |
| [`incomplete-feature-inference`](#incomplete-feature-inference) | analysis, import plans | for every test | source maintainer declares `requires`; nothing removes the warning |
| [`missing-intent-comment`](#missing-intent-comment) | analysis, import plans | when a file does not open with a comment | source maintainer |
| [`unexplained-assertions`](#unexplained-assertions) | analysis, import plans | when a file asserts and has fewer than two comments | source maintainer, optionally |
| [`route-needs-allow-unknown`](#route-needs-allow-unknown) | receiving plans | when a route without `allow_unknown` otherwise matches a test | receiving repository |
| [`accepted-incomplete-inference`](#accepted-incomplete-inference) | receiving plans | for every test a route selects | receiving repository reviews; no change needed |

An import plan repeats the analysis warnings of every test in the snapshots it reads,
including tests the plan does not change.

## Analysis warnings

The analyzer parses a WAST file; it neither validates nor runs it.
A file that does not parse, or has no directives, is an error, not a warning.

### `incomplete-feature-inference`

Stored as `Feature inference is incomplete; syntax evidence is not a complete execution requirement set.`

Every analysis carries this warning and `complete = false`:
no test file can avoid it, and nothing is wrong with a test that has it.
The analyzer records five keywords as syntax evidence and derives nothing else.

| Keyword | Evidence |
| --- | --- |
| `component` | `component-model` |
| `resource` | `resources` |
| `v128` | `simd` |
| `shared` | `threads` |
| `memory64` | `memory64` |

Evidence is a hint for reviewers.
Routing never reads it:
a route selects a test when the route's `supports` includes every entry of the test's `requires`.
Because inference is never complete,
only routes with `allow_unknown = true` receive tests.

- Source maintainer:
  list every capability the test needs in its `requires` in the manifest.
  No change to the test file removes the warning.
- Catalog reviewer:
  check each test's `requires` against its contents before merging an import.
  The import description lists changed tests whose evidence names a feature absent from `requires`;
  start there.
  A keyword inside an assertion that expects rejection is not a requirement.
- Receiving repository:
  set `allow_unknown = true` on a route to receive tests through it.

### `missing-intent-comment`

Stored as `Add a top-of-file comment explaining this test's intent.`

The file has no comment of at least 12 characters before its first token,
not counting comment delimiters and surrounding whitespace.
Reviewers and receiving repositories then cannot tell what the test is meant to check.
Import and delivery proceed regardless.

- Source maintainer:
  start the file with a comment that states what the test checks.
  The next import picks up the new file.
- Catalog reviewer:
  ask the source maintainer.
  Do not edit the copy under `corpus/wast/`; a copy that differs from its recorded digest fails validation.

### `unexplained-assertions`

Stored as `Review whether subtle assertions need nearby explanatory comments.`

The file contains a directive whose keyword starts with `assert_`
and has fewer than two comments, counting the intent comment.
The check counts comments anywhere in the file;
it does not judge whether an assertion needs explaining or whether a comment explains one.
Import and delivery proceed regardless.

- Source maintainer:
  comment the assertions whose expected result is not obvious.
  Leave a self-explanatory test as it is; the warning then stays.
- Catalog reviewer:
  no action is required.

## Receiving warnings

### `route-needs-allow-unknown`

Stored as `route NAME needs explicit allow_unknown for incomplete feature inference`.

The route's `supports`, `covers_any`, and `exclude` accept the test,
but the route does not set `allow_unknown = true`, so the test is not delivered through it.
Because [inference is never complete](#incomplete-feature-inference),
such a route receives no tests at all.

- Receiving repository:
  to receive the tests, set `allow_unknown = true` on the route,
  once its `supports` lists only what your engine implements.
  To decline them, add their identities to the route's or the global `exclude`;
  excluded tests are no longer reported.

### `accepted-incomplete-inference`

Stored as `accepted incomplete feature inference via route NAME`.

A route with `allow_unknown = true` selected the test.
The warning is reported for every selected test, whether or not the plan changes it,
so descriptions count tests per route instead of naming them.
Selection used the `requires` declared by the test's author;
it does not show that your engine supports or passes the test.

- Receiving repository:
  run the proposed tests on your engine before merging,
  and add any you decline to `exclude`.
  No manifest change is needed otherwise.

## Other command warnings

- A failed token revocation is covered under
  [credentials in GitHub Actions](workflow.md#credentials-in-github-actions).
- Skipped release notifications are covered under
  [post-release issue and PR updates](releases.md#post-release-issue-and-pr-updates).
