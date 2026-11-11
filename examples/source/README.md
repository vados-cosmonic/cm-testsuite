# Source example project

This project contributes an integer addition test through `.cm-testsuite.toml`
and receives arithmetic tests from other projects under `tests/shared/`.
Its own test remains authoritative and is never replaced by received files.

Run `dotslash bins/just demo` from the cm-testsuite checkout to copy this project
into `.demo/`, commit its source files, import its tests, and deliver the other
example project's multiplication test. Inspect the generated TOML plans and receipt
there before applying the same workflow to your project.
