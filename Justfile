set positional-arguments

[macos]
set shell := ["zsh", "-eu", "-o", "pipefail", "-c"]

[linux]
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

[windows]
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# An explicit Buck override is a single executable path, including spaces.
buck := if env_var_or_default("BUCK2", "") != "" { quote(env_var("BUCK2")) } else { "dotslash " + quote(justfile_directory() / "bins/buck2") }

# Buck actions use an isolated environment. Forward the discovered Git Bash and
# MSVC environment on Windows so WSL's bash.exe and Git's link.exe cannot win.
host_config := if os() == "windows" {
    '--config "host_tools.bash=$(cygpath -m "$(command -v bash)")"' +
    ' --config "host_tools.path=$(cygpath -mp "$PATH")"' +
    ' --config "host_tools.lib=${LIB:-}" --config "host_tools.include=${INCLUDE:-}"'
} else { "" }

# List the available commands, grouped by purpose.
[group('meta')]
_default:
    @{{quote(just_executable())}} --list

# Check the launcher and supported host before downloading repository tools.
[group('meta')]
_launcher: (_require "dotslash")
    @case '{{os()}}/{{arch()}}' in linux/x86_64|linux/aarch64|macos/x86_64|macos/aarch64|windows/x86_64|windows/aarch64) ;; *) printf '%s\n' 'Unsupported host: {{os()}}/{{arch()}}. See docs/src/building.md for supported hosts.' >&2; exit 1 ;; esac

# Check all build prerequisites without starting a build or installing tools.
[group('meta')]
doctor: _launcher (_require "git" "tar" "xz" "mktemp" "cp" "mkdir" "rm" "dirname" "basename")
    @if [[ "{{os()}}" == windows ]]; then {{quote(just_executable())}} _require cl.exe lib.exe cygpath; else {{quote(just_executable())}} _require cc ar; fi
    @if [[ "{{os()}}" == windows ]] && [[ ! -f "$(dirname -- "$(command -v cl.exe)")/link.exe" ]]; then printf '%s\n' 'MSVC link.exe was not found beside cl.exe. Open Git Bash from the Visual Studio Developer Command Prompt.' >&2; exit 1; fi
    @if [[ "{{os()}}" == windows ]] && [[ -z "${LIB:-}" || -z "${INCLUDE:-}" ]]; then printf '%s\n' 'MSVC LIB and INCLUDE must be set. Open Git Bash from the Visual Studio Developer Command Prompt.' >&2; exit 1; fi
    @if [[ -n "${BUCK2:-}" ]] && ! command -v "$BUCK2" >/dev/null 2>&1; then printf 'BUCK2 executable not found: %s\nSet BUCK2 to an installed executable, or unset it to use the pinned launcher.\n' "$BUCK2" >&2; exit 1; fi
    @printf '%s\n' 'Build prerequisites are available.'

# Report missing commands together before any dependent recipe starts.
[group('meta')]
_require +tools:
    @missing=''; for tool in "$@"; do if ! command -v "$tool" >/dev/null 2>&1; then missing="$missing $tool"; fi; done; if [[ -n "$missing" ]]; then printf 'Missing required tools:%s\n' "$missing" >&2; printf '%s\n' 'Install them using your system package manager and ensure they are on PATH.' 'See docs/src/building.md for prerequisites. No build has been started.' >&2; exit 1; fi

# Build the command entrypoint using Buck's downloaded Rust SDK.
[group('build')]
build: doctor
    {{buck}} build {{host_config}} //:xtask

# Run workspace unit, integration, documentation, and example E2E tests.
[group('test')]
test: doctor
    {{buck}} build {{host_config}} //:tests

# Run Clippy across every workspace target, treating warnings as errors.
[group('test')]
lint: doctor
    {{buck}} build {{host_config}} //:lint

# Check workspace formatting with the pinned rustfmt.
[group('test')]
fmt-check: doctor
    {{buck}} build {{host_config}} //:fmt-check

# Run the same formatting, test, and lint checks used in CI.
[group('ci')]
check: fmt-check test lint

# Run an exchange command; pass --help to see typed command arguments.
[group('meta')]
xtask *args: doctor
    {{buck}} run {{host_config}} //:xtask -- "$@"

# Exercise every example project in isolated local Git repositories.
[group('test')]
demo: doctor
    {{buck}} run {{host_config}} //:xtask -- demo --out .demo

# Regenerate Buck's pinned registry inputs after updating Cargo.lock.
[group('build')]
buck-lock: doctor
    {{buck}} run {{host_config}} //:xtask -- buck-lock

# Build the mdBook with the pinned documentation launcher.
[group('docs')]
docs: doctor
    {{buck}} run {{host_config}} //:xtask -- docs

# Build the documentation and engine-submitted results site.
[group('docs')]
site: doctor
    {{buck}} run {{host_config}} //:xtask -- site

# Audit GitHub workflows without requiring the native C build toolchain.
[group('ci')]
zizmor *args: _launcher
    dotslash {{quote(justfile_directory() / "bins/zizmor")}} "$@" .github
