# Agent instructions

cm-testsuite exchanges WebAssembly component-model WAST tests between projects.
The Rust workspace lives under `rust/`; `xtask` is the only executable.
Buck2 builds it through pinned DotSlash launchers in `bins/`, and Buck-only definitions live under `.buck/`.

## Core rules

- Follow [CONTRIBUTING.md](CONTRIBUTING.md); read the sections relevant to the task.
- Work on a feature branch, never directly on `main`.
- Run repository commands from the checkout root as `dotslash bins/just <recipe>`.
  Start with `dotslash bins/just doctor`; do not install Rust, Buck2, or Just separately.
- Before running Buck in a checkout nested inside another checkout (such as a Git worktree),
  confirm that `dotslash bins/buck2 root --kind project` prints that checkout.
  Otherwise Buck silently builds the outer one;
  see [Building and checking](.agents/skills/cm-testsuite-build/SKILL.md#nested-checkouts).
- Search for existing helpers, fixtures, recipes, and `xtask` subcommands before adding code.
  Reuse or extend them where they fit.
- Run `dotslash bins/just check` before proposing a change, plus the checks the relevant skill lists.
  Re-run them after fixes, and report any check that could not run and why.
- Do not edit generated or snapshot files by hand:
  `.buck/third-party/crates.bzl`, `corpus/wast/`, `metadata/catalog.toml`, `reports/`, and `CHANGELOG.md`.
  Use the command that owns each one.
- Never run the live GitHub suite (`just test-github`), publish, tag, or release unless explicitly asked.
  These create repositories, pull requests, or tags with real credentials.
- Keep conversation, prompt, and iteration history out of code comments, commits, and PR descriptions.
  Keep plans and working notes in the ignored `.llm/` directory.
- Repository instructions and checked-in lint and format configuration take precedence over external skills.

## Task-specific guidance

Read only the skills relevant to the current task, before doing that work.
Combine skills when needed, such as Rust and build guidance for a dependency update.
If the harness does not discover local skills, open the linked `SKILL.md` files directly.

| Task | Skill |
| --- | --- |
| Building, testing, linting, or changing Buck definitions and dependency pins | [Building and checking](.agents/skills/cm-testsuite-build/SKILL.md) |
| Implementing, refactoring, or reviewing Rust code and tests | [Rust changes](.agents/skills/cm-testsuite-rust/SKILL.md) |
| Changing manifests, examples, the curated corpus, the registry, or reports | [Exchange data](.agents/skills/cm-testsuite-exchange/SKILL.md) |
| Changing workflows, shared actions, Just recipes, or tool launchers | [CI and developer automation](.agents/skills/cm-testsuite-automation/SKILL.md) |
| Preparing commits or PRs, or editing documentation | [Contribution conventions](.agents/skills/cm-testsuite-contribution/SKILL.md) |
