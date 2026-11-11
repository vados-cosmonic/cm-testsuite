# Curated WAST

Reviewed imports populate `wast/OWNER/REPOSITORY/NAME.wast`.
The [catalog](../metadata/catalog.toml) records each file's source repository,
immutable revision, and content digest.

> [!WARNING]
> Imported files are snapshots, not authoritative sources.
> Propose corrections in the originating repository, then import the accepted revision.
> Editing a snapshot directly causes its contents to diverge from the catalog digest.

Use `dotslash bins/just xtask validate` from the repository root to check the catalog
against the imported files.
See the [import workflow](../docs/src/workflow.md#import) for acquiring,
planning, and applying updates.
