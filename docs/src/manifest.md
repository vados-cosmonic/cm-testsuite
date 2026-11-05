# Participant manifest

The default location is `.cm-testsuite.toml` at the source repository root. Unknown fields and unsupported schema versions are errors. The manifest and every listed file must be committed; local uncommitted files are not acquisition inputs.

```toml
schema = 1

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
```

`file://tests/a.wast` is relative to the manifest directory; `file:///tests/a.wast` is relative to the tracked repository root. This is project notation, not a standard file URL or access to the importing host's root. Percent encoding is decoded once. Traversal, malformed encodings, backslashes, Git metadata paths, and symlinks are rejected.

Names are unique within a source and use portable ASCII letters, digits, `-`, `_`, `.`, and `/`. `github:owner/repo/name` is the stable identity. Version changes and source-path moves do not change that identity. Source/repository renames and test-name aliases are not automated yet.

A route requires all author-declared `requires` capabilities to be present in `supports`. An empty `covers_any` accepts any category. Global exclusions override all routes. Multiple matching routes are an error; destinations include the source owner/repository and test name to avoid basename collisions. A path declared as an authoritative test cannot be overwritten by delivery, and tests inside receiving destinations cannot be exported as new originals.

The first analyzer reports syntax evidence and **always marks inference incomplete**. It does not claim that syntax inside expected-rejection assertions must be enabled. Default routes therefore withhold uncertain tests. Set `allow_unknown = true` only when you intentionally accept review of those tests; the plan will include warnings. This is metadata selection, not evidence of engine support.

Add a meaningful intent comment at the top of each WAST and explain subtle assertions nearby. The draft emits review warnings, not proof that a comment is correct or sufficient.
