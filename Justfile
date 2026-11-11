buck := env_var_or_default("BUCK2", "./buck2")
export PATH := justfile_directory() + "/.tools:" + env_var("PATH")
set positional-arguments
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]
_default:
    @just --list
bootstrap:
    bash scripts/bootstrap.sh
build: bootstrap
    {{buck}} build //:xtask
test: bootstrap
    {{buck}} build //:tests
lint: bootstrap
    {{buck}} build //:lint
fmt-check: bootstrap
    {{buck}} build //:fmt-check
check: fmt-check test lint
# All exchange operations use the same Rust entrypoint locally and in CI.
xtask *args: bootstrap
    {{buck}} run //:xtask -- "$@"
demo: bootstrap
    {{buck}} run //:xtask -- demo --out .demo
buck-lock: bootstrap
    {{buck}} run //:xtask -- buck-lock
# mdBook is a separately installed, pinned documentation tool; see docs.
docs: bootstrap
    {{buck}} run //:xtask -- docs
site: bootstrap
    {{buck}} run //:xtask -- site

# Audit workflow security with the same pinned version used in CI.
zizmor *args: bootstrap
    ./zizmor "$@" .github
