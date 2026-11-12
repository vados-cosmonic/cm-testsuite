# Enrolled repositories

Each repository in the exchange has one file here, `OWNER/REPO.toml`,
spelled exactly as its `repository` field is:

```toml
# registry/participants/OWNER/REPO.toml
schema = 1
repository = "github:OWNER/REPO"
manifest = ".cm-testsuite.toml" # optional
reference = "main"              # optional; gathered from the default branch when absent
maintainers = ["LOGIN"]         # optional; mentioned when this repository fails
```

Enrolling adds a file and withdrawing deletes it, so enrollments never conflict with each other.
A file that names a different repository than its path is an error,
as is a repository enrolled twice, whatever the case of its name.
Files that do not end in `.toml` are ignored.
See the [workflow guide](../../docs/src/workflow.md#gather-and-distribute) for the commands that read these files.
