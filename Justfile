set positional-arguments
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# An explicit Buck override is a single executable path, including spaces.
buck := if env_var_or_default("BUCK2", "") != "" { quote(env_var("BUCK2")) } else { "dotslash " + quote(justfile_directory() / "buck2") }

_default:
    @{{quote(just_executable())}} --list

# Check the launcher environment before downloading or running repository tools.
_launcher: (_require "dotslash")
    @if [[ '{{os()}}/{{arch()}}' != 'linux/x86_64' ]]; then printf '%s\n' 'Unsupported build host: {{os()}}/{{arch()}}. The pinned SDK currently supports Linux x86_64; use a Linux x86_64 environment.' >&2; exit 1; fi

# Check system prerequisites without installing tools or starting a build.
doctor: _launcher (_require "git" "cc" "ar" "tar" "xz" "realpath" "mktemp" "cp" "mkdir" "rm")
    @if [[ -n "${BUCK2:-}" ]] && ! command -v "$BUCK2" >/dev/null 2>&1; then printf 'BUCK2 executable not found: %s\nSet BUCK2 to an installed executable, or unset it to use the pinned launcher.\n' "$BUCK2" >&2; exit 1; fi
    @printf '%s\n' 'Build prerequisites are available.'

# Report every missing command together so setup does not fail one tool at a time.
_require +tools:
    #!/usr/bin/env bash
    set -euo pipefail
    missing=()
    for tool in "$@"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing+=("$tool")
        fi
    done
    if ((${#missing[@]})); then
        printf 'Missing required tools: %s\n' "${missing[*]}" >&2
        printf '%s\n' 'Install them using your system package manager and ensure they are on PATH.' 'See docs/src/building.md for prerequisites. No build has been started.' >&2
        exit 1
    fi

build: doctor
    {{buck}} build //:xtask

test: doctor
    {{buck}} build //:tests

lint: doctor
    {{buck}} build //:lint

fmt-check: doctor
    {{buck}} build //:fmt-check

check: fmt-check test lint

# All exchange operations use the same Rust entrypoint locally and in CI.
xtask *args: doctor
    {{buck}} run //:xtask -- "$@"

demo: doctor
    {{buck}} run //:xtask -- demo --out .demo

buck-lock: doctor
    {{buck}} run //:xtask -- buck-lock

# mdBook is a separately downloaded, pinned documentation tool; see docs.
docs: doctor
    {{buck}} run //:xtask -- docs

site: doctor
    {{buck}} run //:xtask -- site

# Workflow auditing needs DotSlash, but not the native build prerequisites.
zizmor *args: _launcher
    dotslash {{quote(justfile_directory() / "zizmor")}} "$@" .github
