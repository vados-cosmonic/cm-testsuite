"""Build the actual Cargo graph with declared, checksum-pinned, offline inputs."""
load("@wasmono//:defs.bzl", "host_os")

def _cargo_impl(ctx):
    source = ctx.actions.symlinked_dir("source", ctx.attrs.srcs)
    output = ctx.actions.declare_output(("xtask.exe" if host_os() == "windows" else "xtask") if ctx.attrs.mode == "build" else ctx.attrs.mode + ".log")
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
    ctx.actions.run(command, category = "cargo_" + ctx.attrs.mode, local_only = True, env = ctx.attrs.env)
    providers = [DefaultInfo(default_output = output)]
    if ctx.attrs.mode == "build":
        providers.append(RunInfo(args = cmd_args(output)))
    return providers

cargo_workspace = rule(
    impl = _cargo_impl,
    attrs = {
        "bash": attrs.string(),
        "env": attrs.dict(attrs.string(), attrs.string()),
        "srcs": attrs.dict(attrs.string(), attrs.source()),
        "sdk": attrs.exec_dep(default = "toolchains//rust:sdk"),
        "crates": attrs.dict(attrs.string(), attrs.dep()),
        "checksums": attrs.dict(attrs.string(), attrs.string()),
        "mode": attrs.enum(["build", "test", "lint", "fmt"]),
        "script": attrs.source(default = "//build-support:cargo.sh"),
    },
)
