"""Build the actual Cargo graph with declared, checksum-pinned, offline inputs."""
load("@wasmono//:defs.bzl", "host_os")

def _cargo_impl(ctx):
    source = ctx.actions.symlinked_dir("source", ctx.attrs.srcs)
    output = ctx.actions.declare_output(
        "artifacts" if ctx.attrs.mode == "build" else ctx.attrs.mode + ".log",
        dir = ctx.attrs.mode == "build",
    )
    # Keep the native command below Windows' length limit while retaining every
    # registry archive as a declared, hashed input to the Cargo action.
    crate_lines = []
    for name, dep in ctx.attrs.crates.items():
        crate_lines.extend([name, dep[DefaultInfo].default_outputs[0], ctx.attrs.checksums[name]])
    crate_lines.append("")
    manifest, extra_inputs = ctx.actions.write(
        "crates.args",
        cmd_args(crate_lines, delimiter = "\n"),
        allow_args = True,
        with_inputs = True,
    )
    command = cmd_args(
        ctx.attrs.bash,
        ctx.attrs.script,
        ctx.attrs.sdk[DefaultInfo].default_outputs[0],
        source,
        ctx.attrs.mode,
        output.as_output(),
        manifest,
        hidden = extra_inputs,
    )
    ctx.actions.run(command, category = "cargo_" + ctx.attrs.mode.replace("-", "_"), local_only = True, env = ctx.attrs.env)
    return [DefaultInfo(default_output = output)]


def _cargo_binary_impl(ctx):
    bundle = ctx.attrs.artifacts[DefaultInfo].default_outputs[0]
    executable = bundle.project("xtask" + (".exe" if host_os() == "windows" else ""))
    return [DefaultInfo(default_output = executable), RunInfo(args = cmd_args(executable))]


cargo_binary = rule(
    impl = _cargo_binary_impl,
    attrs = {"artifacts": attrs.dep()},
)


def _cargo_test_impl(ctx):
    bundle = ctx.attrs.artifacts[DefaultInfo].default_outputs[0]
    executable = bundle.project("xtask" + (".exe" if host_os() == "windows" else ""))
    command = cmd_args(
        executable, "buck-artifacts", "test",
        "--bundle", bundle,
        "--sdk", ctx.attrs.sdk[DefaultInfo].default_outputs[0],
        "--suite", ctx.attrs.suite,
    )
    # buck run exposes the explicit live suite; normal test suites never include
    # it. Every invocation executes tests, even when compilation is a cache hit.
    if ctx.attrs.suite == "live":
        return [DefaultInfo(default_output = bundle), RunInfo(args = cmd_args(command, "--"))]
    return [
        DefaultInfo(default_output = bundle),
        ExternalRunnerTestInfo(
            type = "custom",
            command = [cmd_args(command, "--")],
            env = ctx.attrs.env,
            use_project_relative_paths = False,
            supports_test_execution_caching = False,
        ),
    ]


cargo_test = rule(
    impl = _cargo_test_impl,
    attrs = {
        "artifacts": attrs.dep(),
        "env": attrs.dict(attrs.string(), attrs.string()),
        "sdk": attrs.exec_dep(default = "toolchains//rust:sdk"),
        "suite": attrs.enum(["unit", "integration", "doc", "live"]),
    },
)


cargo_workspace = rule(
    impl = _cargo_impl,
    attrs = {
        "bash": attrs.string(),
        "env": attrs.dict(attrs.string(), attrs.string()),
        "srcs": attrs.dict(attrs.string(), attrs.source()),
        "sdk": attrs.exec_dep(default = "toolchains//rust:sdk"),
        "crates": attrs.dict(attrs.string(), attrs.dep()),
        "checksums": attrs.dict(attrs.string(), attrs.string()),
        "mode": attrs.enum(["build", "lint", "fmt"]),
        "script": attrs.source(default = "//.buck/build-support:cargo.sh"),
    },
)
