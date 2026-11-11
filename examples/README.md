# Example projects

Each immediate subdirectory is a standalone project that contributes checked-in
WAST tests and receives tests from the other projects through cm-testsuite.
Copy a project's `.cm-testsuite.toml` into your repository and adjust its test
paths, metadata, and receiving routes to match your project.

Run `dotslash bins/just demo` from the checkout root. The command discovers every
project directory, copies it into `.demo/`, and exercises acquisition, catalog
import, validation, and receiving. It verifies exact delivered bytes, preservation
of original files, and empty plans on repeated runs. Source examples stay untouched.

See [CONTRIBUTING.md](../CONTRIBUTING.md#adding-examples-and-tests) for adding
example projects and running their E2E checks.
