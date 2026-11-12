# Participant manifest

The default location is `.cm-testsuite.toml` at the source repository root.
Unknown fields and unsupported schema versions are errors.
A repository can share its own tests, receive tests from other repositories,
or do both.

## Example

```toml
schema = 1

[receive]
enabled = true
additions = "pull-request"
updates = "pull-request"
deletions = "manual"
exclude = []

[[receive.route]]
name = "resources"
destination = "file:///tests/shared/resources"
supports = ["component-model", "resources"]
covers_any = ["resource-lifetime"]
exclude = []
allow_unknown = false

[[test]]
name = "resources/borrow-after-drop"
path = "file:///tests/resources/borrow-after-drop.wast"
summary = "Reject using a borrow after its owner is dropped"
requires = ["component-model", "resources"]
covers = ["resource-lifetime"]
# Optional full immutable Git commit ID:
# revision = "..."
# Optional lowercase SHA-256 of the exact file bytes:
# sha256 = "..."
```

## Shared path rules

`file://tests/a.wast` is relative to the manifest directory.
`file:///tests/a.wast` is relative to the tracked repository root.
This is project notation, not a standard file URL or access to the importing
host's root.

Percent encoding is decoded once.
Traversal, malformed encodings, backslashes, Git metadata paths, and symlinks
are rejected.
These rules apply to both exported test paths and receiving destinations.

## For authors sharing tests

### Committed sources and stable identities

Commit the manifest and every listed WAST file before requesting an import.
Acquisition reads committed snapshots; uncommitted files are not inputs.
Use the optional `revision` field to pin a test to a full immutable commit ID,
and `sha256` to require an exact content digest.

Test names must be unique within a source.
Use portable ASCII letters, digits, `-`, `_`, `.`, and `/`.
The combination `github:owner/repo/name` is the test's stable identity.
Keep the name when moving a source file or updating its contents.
Repository renames and test-name aliases are not handled automatically.

### Requirements, coverage, and intent

List the capabilities needed by a test in `requires` and its coverage categories
in `covers`; receiving repositories use these declarations to select tests.
Do not infer execution requirements solely from syntax in assertions that expect
rejection of invalid or malformed inputs.

Add a meaningful intent comment at the top of each WAST file and explain subtle
assertions nearby.
The analyzer can flag missing explanatory comments, but authors must check that
the explanations accurately describe the test.
[Warnings](warnings.md) explains each warning the analyzer reports and how to resolve it.

Export original tests from their authoritative locations.
Tests inside receiving destinations cannot be exported as new originals.

## For repositories receiving tests

### Capabilities and routing

Set `[receive].enabled = true` to opt in and define `[[receive.route]]` entries
for the destinations and test categories you want.
Each route must list every author-declared `requires` capability in `supports`.
An empty `covers_any` accepts any coverage category.

A test must match at most one route; multiple matching routes are an error.
Destinations include the source owner, repository, and test name to avoid basename
collisions.
Delivery cannot overwrite a path declared as an authoritative test in your manifest.

### Exclusions and file lifecycle

Global `[receive].exclude` identities override all routes.
A route's `exclude` list applies to that route.
Exclusions also preserve previously received files when their source retires them.

Use `additions`, `updates`, and `deletions` to choose between pull-request proposals
and manual handling of each kind of change.
Source retirement does not automatically delete received files;
deletions require an explicit pull-request policy.

Receipts remember the receiving route by name, so keep that name when moving
its destination.
For older receipts or renamed routes, the delivered path is used to identify
the route.
If that is ambiguous, an exclusion on any route conservatively prevents retirement.

### Incomplete feature analysis

The analyzer reports syntax evidence and **always marks inference incomplete**.
Default routes therefore withhold tests whose requirements may be incomplete.
Set `allow_unknown = true` only when you intend to review those tests despite
that uncertainty; the plan will include warnings.
A route without it reports [`route-needs-allow-unknown`](warnings.md#route-needs-allow-unknown)
for each test it withholds;
a route with it reports [`accepted-incomplete-inference`](warnings.md#accepted-incomplete-inference)
for each test it selects.

Routing selects tests using metadata.
It does not establish that your engine supports or passes the selected tests.
