# Example projects

Each immediate subdirectory is a standalone project that contributes checked-in
WAST tests and receives tests from the other projects through cm-testsuite.
Copy a project's `.cm-testsuite.toml` into your repository and adjust its test
paths, metadata, and receiving routes to match your project.

Run `dotslash bins/just demo` from the checkout root. The command discovers every
project directory, copies it into `.demo/`, and exercises acquisition, catalog
import, validation, and receiving. It verifies exact delivered bytes, preservation
of original files, and empty plans on repeated runs. Source examples stay untouched.

`dotslash bins/just test` runs this flow as an E2E test. New example directories
are enrolled automatically: each must have a valid `.cm-testsuite.toml`, contribute
at least one test, and opt into receiving another project's tests.
