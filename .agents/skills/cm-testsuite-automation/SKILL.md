---
name: cm-testsuite-automation
description: Create or review cm-testsuite GitHub Actions workflows, shared composite actions, Just recipes, or DotSlash tool launchers; use for automation changes rather than merely running existing checks.
---

# CI and developer automation

Read the sections that match the files being changed.
[MAINTAINERS.md](../../../MAINTAINERS.md#maintaining-ci-and-tool-pins) describes the maintenance policy.

## Task tooling

- Put logic in an `xtask` subcommand that CI calls and that runs the same way locally.
  Do not inline multi-step logic in a workflow or add a shell script.
- Expose it through a recipe in the `Justfile`, with a comment line and a `[group(...)]` attribute.
  Recipes that invoke Buck depend on `doctor`.
- Recipes must work under zsh on macOS and Bash on Linux and Windows.
  Quote paths, and pass `{{host_config}}` to every Buck invocation so Windows forwards its compiler environment.
- Node.js is used only for commitlint. Do not add other JavaScript tooling.

## Workflows

- Start every workflow with `permissions: {}` and grant each job only what it needs.
- Pin every action to a full commit SHA with the version in a trailing comment.
- Set `persist-credentials: false` on checkouts unless the job pushes.
- Write YAML lists one item per line, never in the inline `[a, b]` form.
- Keep tool installation and caching in `.github/actions/setup-tools` and Buck setup in
  `.github/actions/setup-buck2`, so every workflow shares one implementation.
  Update the cache keys there when a tool pin changes.
- Workflows that publish, tag, or release are manually invoked or tightly guarded.
  Do not widen their triggers or permissions without being asked.
- Audit every workflow or action change with `dotslash bins/just zizmor --offline`.
  Fix findings instead of suppressing them.

## Tool launchers

- Each file in `bins/` is a DotSlash descriptor listing a download and digest per platform.
  Cover every supported host when changing one.
- Take sizes and digests from the upstream release; never invent or reuse them.
- Update the [pinned-tools table](../../../docs/src/building.md#pinned-tools)
  and [THIRD_PARTY.md](../../../THIRD_PARTY.md) to match.
