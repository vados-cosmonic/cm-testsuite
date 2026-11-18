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
exclude = []

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

WAST files can declare structured metadata before the first directive:

```wat
(@cm-testsuite
  (schema 1)
  (intent "Verify that a borrowed resource cannot outlive its owner.")
  (requires "component-model" "resources")
  (covers "resource-lifetime"))
```

This header describes the entire file, including scripts with several modules or
components. License notices and other comments may precede it.
Annotations use the [WebAssembly annotation syntax](https://webassembly.github.io/spec/core/text/lexical.html#annotations).
Tools that do not recognize `@cm-testsuite` can ignore it.

The contract is:

- At most one header, before any WAST directive; a header inside a module or
  component is an error. A header does not replace the test's directives.
- `schema` is required and must be `1`.
- `intent` is required and contains one nonblank UTF-8 string.
  Describe the behavior established by the assertions, rather than just the filename.
- `requires` and `covers` are optional lists of nonblank UTF-8 strings.
  Unknown labels are accepted, duplicates are normalized, and empty lists are valid.
  These are author-supplied descriptions, not verified feature requirements.
- Duplicate fields, unknown fields, malformed values, and unsupported schemas
  are errors. Field order is unrestricted.
- Optional `(extensions (@vendor.example ...))` holds other tools' annotations.
  Extension payloads are retained in the original file but are not interpreted
  or copied into structured metadata. Other annotation IDs elsewhere are ignored
  for metadata and syntax evidence.

WAT string escaping and nested comments work normally.
Annotation-looking text in comments, strings, or an unrelated annotation is not
interpreted as a header. This includes quoted malformed-module payloads.

Acquisition uses the header's intent as the catalog's existing `summary` field.
A present `requires` or `covers` field replaces the corresponding manifest value,
including when the header explicitly supplies an empty list.
An omitted field keeps the manifest value.
The extracted declarations also appear under `analysis.metadata`, preserving the
named `intent` and the distinction between absent and empty label lists.
Snapshots and catalog validation check that these declarations match the WAST bytes
and resolved catalog fields.

Unannotated files remain valid and use manifest metadata.
The analyzer warns about the missing header and retains its guidance about intent
comments and subtle assertions. Human review must still check whether the intent
accurately describes the test.

`requires` and `covers` never gate acquisition or delivery, configure an engine,
or make feature inference complete. Component Model and engine capabilities do
not have a shared exhaustive vocabulary. Core Wasm feature names may be useful
labels, but this schema does not interpret them as enable/disable instructions.
In particular, syntax in negative assertions does not establish execution requirements.

Inspect a file's metadata, syntax evidence, and warnings with:

```sh
dotslash bins/just xtask analyze path/to/test.wast
```

Export original tests from their authoritative locations.
Tests inside receiving destinations cannot be exported as new originals.

## For repositories receiving tests

### Capabilities and routing

Set `[receive].enabled = true` to opt in and define `[[receive.route]]` entries
for the destinations you want.
`supports` and `covers_any` remain accepted as advisory fields: a mismatch produces
a plan warning, but does not exclude a test. `allow_unknown` remains accepted for
compatibility and has no effect on selection.

A test must match at most one route after explicit exclusions;
multiple matching routes are an error.

> [!IMPORTANT]
> Feature labels and inferred completeness no longer filter receiving routes.
> Existing configurations that partitioned tests using `supports` or `covers_any`
> must use a single destination or explicit route exclusions to avoid ambiguity.
> Review receiving plans before applying them; tests previously withheld by feature
> filters may now be proposed.

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
Incomplete inference produces plan warnings and never withholds tests.
Author-declared metadata does not change inference completeness.
Receiving a test does not establish that your engine supports or passes it.

When upgrading from an older analyzer, reacquire sources and apply new import plans
to refresh recorded analysis before validation or distribution.
Use the recorded source revisions to refresh analysis without advancing test content.
