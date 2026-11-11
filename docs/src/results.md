# Engine-submitted results

Engines execute tests in their own projects.
cm-testsuite validates their submitted observations without executing WAST or certifying conformance.

## Report format

```json
{
  "schema": 1,
  "publisher": "your-approved-publisher",
  "engine": "your-engine",
  "engine_version": "your-build",
  "run_id": "your-ci-run",
  "catalog_sha256": "replace-with-the-validated-catalog-digest",
  "reported_at": "2026-11-05T00:00:00Z",
  "platform": "linux-x86_64",
  "configuration": "describe-enabled-features",
  "results": []
}
```

### Report fields

| Field | Meaning |
| --- | --- |
| `schema` | Report schema version; use `1`. |
| `publisher` | Identifier approved in `registry/publishers.json`. |
| `engine` | Name of the engine that ran the tests. |
| `engine_version` | Engine version or build identifier. |
| `run_id` | Identifier for the submitting harness or CI run. |
| `catalog_sha256` | Digest of the exact catalog snapshot used for the run. |
| `reported_at` | Timestamp supplied by the publisher. |
| `platform` | Operating system and architecture used for the run. |
| `configuration` | Engine settings and enabled features. |
| `results` | Array of observations for tests in the catalog; omitted tests are not reported. |

### Per-test fields

| Field | Meaning |
| --- | --- |
| `id` | Stable test identity, such as `github:owner/repo/name`. |
| `sha256` | SHA-256 of the exact WAST file bytes tested. |
| `status` | One of the supported statuses below. |
| `diagnostic` | Optional explanatory text from the harness. |

### Statuses

| Status | Meaning |
| --- | --- |
| `pass` | The harness observed the expected result. |
| `fail` | The harness observed a result that did not meet the test's expectations. |
| `skip` | The harness skipped the test. |
| `unsupported` | The engine or harness does not support the test's requirements. |
| `timeout` | Execution exceeded the harness's time limit. |
| `infrastructure-error` | A problem with the test environment prevented a result. |

A successful expected-negative assertion should be reported as `pass`
by the engine's harness.

## Submitting a report


Publisher identifiers in `registry/publishers.json` should be approved during review.

To run admission checks locally:

```sh
./just xtask results-import --report /path/to/report.json
./just site
```

## Storage and display

Accepted reports are content-addressed under `reports/CATALOG_DIGEST/REPORT_DIGEST.json`, alongside the catalog snapshot.
The site displays all runs submitted for the current catalog.
Older reports are retained on disk but not mixed into the current comparison.
Missing entries mean **not reported**.
If there are no current reports, the page says so; it generates no sample execution data.

The current publisher allowlist is an admission policy, not a cryptographic signature or independent verification of who produced a file.
Review report submissions and their provenance before merging them.
Timestamps are displayed as supplied; the site does not infer a latest run or compare reports chronologically.
