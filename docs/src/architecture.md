# Architecture

The workspace lives under `rust/`. Library boundaries follow responsibilities in the exchange flow. There is no separate cm-testsuite application binary; xtask is the executable task entrypoint.

| Crate | Responsibility |
| --- | --- |
| `cm-catalog` | Manifest/schema validation, repository paths, identity, hashes, source snapshots, catalog and retirements |
| `cm-github` | Caller-token REST transport and regular-file reads through Git trees |
| `cm-wast-analysis` | WAST parsing, syntax evidence, intent-comment warnings, explicit analysis incompleteness |
| `cm-acquisition` | Immutable GitHub/local-Git acquisition and complete source snapshots |
| `cm-exchange-plan` | Read-only import/receive plans, routes, exclusions, baseline checks, and delivery receipts |
| `cm-delivery` | Guarded local application and explicit GitHub draft-PR reconciliation |
| `cm-results` | Submitted-report validation and HTML presentation |
| `xtask` | Argument parsing, configuration/credentials, task wiring, progress and exit reporting |

Libraries accept explicit inputs and return structured values/errors. They do not depend on xtask or GitHub Actions. Catalog types support analysis/acquisition/planning; delivery consumes plans; results consumes the catalog. The GitHub transport is shared by acquisition and delivery, with an injectable API boundary for tests.

An acquisition snapshot contains exact source bytes plus provenance. It is not applied automatically. A plan records each expected prior file digest, proposed new bytes or deletion, and policy/catalog guards. Application checks the full plan before mutation. Only registered sources supplied in a successful snapshot can cause central retirements; fetch errors are errors, not empty sources.

A source/name identity is independent of its Git revision. The catalog holds the file SHA-256, source file and commit, and the manifest commit/hash. Source-path moves preserve a declared name. Identity-name changes need manual treatment; alias migrations are not implemented.

Received identities and file locations are stored in `.cm-testsuite-lock.toml`. Exclusion stops future additions/updates and leaves an existing file intact. Source retirement triggers receiving deletion only when enabled. Every removal or move checks the managed baseline.

The transport uses Octocrab for GitHub and the existing Git executable to read local committed object databases. No upstream build scripts or generated WAST are invoked. Filesystem symlinks and Git symlink/submodule entries are rejected.
