# Submitted results

Accepted engine reports are stored under `<catalog-digest>/<report-digest>.json`.
Each directory also contains the corresponding `catalog.json` snapshot.

Use `./just xtask results-import --report PATH` from the repository root to
validate a report against the current catalog and publisher allowlist, then store it.
The results site displays reports for the current catalog; older reports remain
available in their catalog directories.

See the [result-submission guide](../docs/src/results.md) for the report format
and publisher requirements.
