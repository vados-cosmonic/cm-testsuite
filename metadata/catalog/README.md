# Catalog records

Each source repository in the catalog has one file here, `OWNER/REPO.toml`,
spelled exactly as its source name is.
The file records the commit the source was last read at, its tests, and its retired identities:

```toml
# metadata/catalog/OWNER/REPO.toml
schema = 1

[sources."github:OWNER/REPO"]
revision = "<commit>"
manifest_path = ".cm-testsuite.toml"
manifest_sha256 = "<sha256>"

[tests."github:OWNER/REPO/NAME"]
# source, revision, digest, corpus path, and analysis of one test

[retired]
"github:OWNER/REPO/OLD-NAME" = "<sha256>"
```

Import plans write these files; do not edit them by hand.
An import replaces only the files of the sources it covers,
so imports of different sources never conflict with each other.
A file that records a different source than its path names is an error,
as is a source recorded twice, whatever the case of its name.
Files that do not end in `.toml` are ignored.
`../catalog.toml` is still read and may record any number of sources;
a source recorded there moves to its own file the next time it is imported.
See the [workflow guide](../../docs/src/workflow.md#import) for the commands that write these files.
