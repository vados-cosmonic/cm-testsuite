# Engine-submitted results

Engines execute tests in their own projects. cm-testsuite validates their submitted observations without executing WAST or certifying conformance.

A report includes `schema`, `publisher`, `engine`, `engine_version`, `run_id`, `catalog_sha256`, `reported_at`, `platform`, `configuration`, and `results`. Each result has a stable `id`, file `sha256`, `status`, and optional `diagnostic`. Supported statuses are `pass`, `fail`, `skip`, `unsupported`, `timeout`, and `infrastructure-error`. A successful expected-negative assertion should be reported as `pass` by the engine's harness.

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

Approve publisher identifiers in `registry/publishers.json` through review. Admission checks the publisher, exact catalog digest, membership, per-file digest, duplicate identities, and required metadata. Run locally:

```sh
./just xtask results-import --report /path/to/report.json
./just site
```

Accepted reports are content-addressed under `reports/CATALOG_DIGEST/REPORT_DIGEST.json`, alongside the catalog snapshot. The site displays all runs submitted for the current catalog. Older reports are retained on disk but not mixed into the current comparison. Missing entries mean **not reported**. If there are no current reports, the page says so; it generates no sample execution data.

The current publisher allowlist is an admission policy, not a cryptographic signature or independent verification of who produced a file. Review report submissions and their provenance before merging them. Timestamps are displayed as supplied; the draft does not infer a latest run or compare reports chronologically.
