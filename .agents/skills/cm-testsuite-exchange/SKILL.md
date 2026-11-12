---
name: cm-testsuite-exchange
description: Change cm-testsuite exchange data, including .cm-testsuite.toml manifests, example projects under examples/, the curated WAST corpus, the catalog, the publisher registry, or submitted result reports.
---

# Exchange data

Most of these files are written by `xtask` commands and checked against recorded digests.
Change them through the owning command, not by hand.

| Path | Owner | How to change it |
| --- | --- | --- |
| `corpus/wast/` | Import plans | Fix the test in its source repository, then acquire, plan, and apply an import |
| `metadata/catalog.toml` | Import plans | Same as the corpus; it records each file's source, revision, and digest |
| `reports/` | `xtask results-import` | Admit a report with the command; never write one directly |
| `registry/publishers.toml` | Maintainer review | Edit by hand only as an explicitly requested admission change |
| `registry/participants.toml` | `xtask enroll` | Add an entry with `dotslash bins/just enroll --issue NUMBER --apply`; edit by hand only for an explicitly requested change the command cannot make |
| `examples/*/` | Hand-written | Edit directly; see below |
| `.cm-testsuite.toml` | Hand-written | Follow the [manifest guide](../../../docs/src/manifest.md) |

- Editing an imported snapshot makes it diverge from its catalog digest and fails validation.
- Run `dotslash bins/just xtask validate` after any change to the corpus, catalog, or manifests.
- `.wat` and `.wast` files are stored without line-ending conversion because digests cover exact bytes.
  Do not reformat or normalize them.
- The [workflow guide](../../../docs/src/workflow.md) documents acquire, plan-import, plan-receive, and apply.
  Planning is read-only; `apply` mutates the checkout; `publish` opens a draft pull request on GitHub.
  Run `publish` only when explicitly asked.
- `gather` and `distribute` run those flows for every enrolled repository and only report by default.
  Pass `--apply`, `--publish`, or `--report-to` only when explicitly asked;
  the last two open pull requests and issues on GitHub.
- `enroll` reads an enrollment issue and only reports by default.
  Its `--publish` opens a pull request; pass it only when explicitly asked.
  The form's labels in `.github/ISSUE_TEMPLATE/enroll-repository.yml` and the parser
  in `rust/crates/delivery/src/enroll/form.rs` must change together.

## Examples

- Every directory immediately under `examples/` is discovered automatically as an end-to-end fixture.
- A new example needs a minimal README, a manifest contributing at least one WAST test,
  and a receiving route that accepts another example's test.
- Run `dotslash bins/just demo` after changing one.
  It copies the examples into the ignored `.demo/` directory and leaves the sources untouched.
