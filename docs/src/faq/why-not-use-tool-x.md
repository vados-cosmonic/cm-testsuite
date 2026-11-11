# Why build library crates instead of adopting an existing tool?

cm-testsuite curates individually versioned WAST files from multiple authoritative repositories. It needs stable test identities, exact-byte provenance, feature-aware receiving routes, persistent exclusions, and reviewed GitHub changes. Every operation must be available locally, and the acquisition and synchronization implementation must be Rust.

We evaluated existing tools before choosing project-owned library crates. Some tools successfully perform parts of the job. The decision to build those parts here reflects the combination of requirements, integration costs, and language constraints; it is not a claim that those tools are defective.

## Why library crates and xtask?

The libraries follow the workflow: discovery and acquisition, catalog and provenance, WAST analysis, receiving/change planning, delivery, and submitted results. Their APIs work independently of command-line arguments, GitHub Actions, and the current working directory. xtask is the sole command entrypoint: it interprets arguments, supplies configuration and credentials, calls the libraries, and presents results. There is no separate application CLI crate.

We reuse upstream Rust libraries for generic capabilities such as HTTP, serialization, hashing, and parsing. Project-owned crates implement the test-exchange semantics and compose those dependencies. We do not need a separate crate for every command or every rejected upstream tool; exact crate boundaries follow cohesive responsibilities and useful reuse.

## Why not vendir?

[Vendir](https://github.com/carvel-dev/vendir) successfully gathered multiple sources and independently pinned files in our experiments. It is implemented in Go, so it does not meet our Rust requirement. Invoking it from xtask would not change that.

It also manages directory contents rather than our test identities and review state. An experiment confirmed that syncing overwrites local edits in managed files. Our Rust acquisition layer fetches into staging, verify provenance, and prepare a change plan before applying anything. Vendir's successful pinning and idempotence cases remain useful acceptance criteria.

## Why not Copybara?

[Copybara](https://github.com/google/copybara) demonstrated imports, second-hop delivery, independent file pins, deletions, and reverse proposal branches. Its migration and PR functionality is substantial. Its Java implementation excludes it under our language constraint.

There is also an integration issue specific to our changing selections: adding an exclusion caused Copybara's last-revision consistency guard to reject the next migration, because the previous snapshot no longer matched the new rules. This guard is useful protection, but adapting its baseline model to our receiving policies would require additional orchestration. Our delivery library must explicitly handle route changes, exclusions, and local divergence. GitHub PR behavior was inspected, not tested live, during this evaluation.

## Why not Josh?

[Josh](https://github.com/josh-project/josh) is Rust and its reversible Git filtering worked. We demonstrated filtered views, two-source aggregation, per-file pinning, conflict detection, and a correction returned to its source using an explicit unfiltered base.

The exchange needs curated file snapshots more than transformed repository histories. Josh does not supply stable test identity, the provenance catalog, feature routing, receiving exclusions, or approval policy. Expressing independent file pins through revision-dependent filters adds machinery without removing those responsibilities. Its client-side filtering also downloads the full upstream object database; it is not a substitute for targeted acquisition.

We therefore do not plan to depend on Josh for file exchange. It could become useful later if consumers need a tests-only mirror or reversible synchronization of a coherent directory. That would be a new requirement to evaluate, not a reason to carry the dependency now.

## Why not rustc-josh-sync?

[rustc-josh-sync](https://github.com/rust-lang/josh-sync) solves synchronization problems for Rust's repositories. The inspected version allows a configurable pull upstream, but its push flow still assumes Rust repository names and relationships. Generalizing it would require upstream changes or a maintained adaptation. We also did not find a licensing declaration in the inspected checkout, which would need clarification before extracting code.

## Why not the WPT synchronization tools?

[Mozilla wpt-sync](https://github.com/mozilla/wpt-sync) and [Chromium's WPT integration](https://chromium.googlesource.com/chromium/src/+/main/docs/testing/web_platform_tests.md) establish useful patterns for source authority, imported copies, and corrections flowing upstream. Their implementations depend on Gecko/Chromium infrastructure, issue tracking, review systems, and service deployment. Adapting them is a substantial port, not configuration of a generic exchange service.

Our libraries will retain the separation of source ownership, received copies, and local metadata, while using the project's own manifests and review policies.

## Why not Test262 tooling?

[Test262's harness](https://github.com/tc39/test262-harness) handles JavaScript test discovery, harness includes, execution variants, and host execution. cm-testsuite collects WAST and does not execute tests. Its feature metadata and negative-test conventions are useful references, but its runner does not implement our exchange workflow.

Future Test262 or WPT adapters should preserve native metadata and companion files. The generic exchange libraries should not require translating those formats into WAST or taking over their execution.

## Why not WebAssembly/testsuite's updater or jco's fixture sync?

[WebAssembly/testsuite](https://github.com/WebAssembly/testsuite) already aggregates tests, but its updater assumes particular WebAssembly repositories and core/proposal layouts. A layout probe showed that it omitted a Component Model resource-test directory and flattened nested core filenames into colliding destinations. Those assumptions can suit its existing corpus; arbitrary participant manifests need different behavior.

The inspected jco fixture-sync code is a closer Rust example: it supports explicit file pins, byte comparison, check mode, and fetching all inputs before writes. However, it is private xtask code with fixed locations and a curl subprocess, rather than an independently reusable exchange library. Its behavior informs our acquisition design. We should still evaluate extracting reusable pieces instead of copying or rewriting them automatically.

## Why not subtrees or submodules?

They manage repository or directory relationships. They do not supply per-test metadata, mixed source revisions, receiving policy, or central review state. Storage uses exact-byte copies with a provenance catalog. Consumers can still submodule cm-testsuite and use its `corpus/wast/` directory.

## Why not an existing GitHub PR action or the GitHub CLI?

An action such as [create-pull-request](https://github.com/peter-evans/create-pull-request) provides useful PR machinery, but does not give us the standalone local Rust API and test-specific reconciliation behavior we need. The Go-based [GitHub CLI](https://github.com/cli/cli) also does not meet the language requirement for this backend. We use a Rust HTTP/API layer with caller-supplied tokens; CI invokes the same xtask commands used locally.

## Why not wpt.fyi for the results site?

[wpt.fyi](https://github.com/web-platform-tests/wpt.fyi) consumes externally produced results, which matches our division of responsibility. Its App Engine backend and browser-oriented model are not a static GitHub Pages component for our catalog. We validate engine-submitted TOML and generate a separate results page, while using mdBook for documentation.

## Why not Reindeer-generated Buck targets yet?

[Reindeer](https://github.com/facebookincubator/reindeer) remains a possible build integration, not a rejected exchange backend. The build invokes Cargo inside Buck actions with a downloaded Rust SDK and lockfile-derived, checksum-pinned crate inputs. This keeps one feature/build-script/proc-macro graph while the workspace interfaces are changing. The tradeoff is coarse build caching and test discovery. Moving to native per-crate Buck rules through Reindeer could improve those properties; it would need to preserve the same Cargo graph. This choice does not imply that Reindeer cannot support the workspace.

## What do we still reuse?

The `wast` parser, Buck2, Just, mdBook, relevant Wasmono rules, and wasi-testsuite's downloaded Rust toolchain approach remain useful. The feature detector, identity model, receiving policy, and reconciliation logic remain our responsibility. Further upstream Rust dependencies should be selected and tested at those library boundaries.

These decisions can change if an upstream project acquires a suitable interface or our requirements change. Reconsideration should compare supported behavior, maintenance cost, and integration effort against the library we would otherwise maintain.
