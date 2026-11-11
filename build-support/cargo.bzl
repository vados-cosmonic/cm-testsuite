"""Build the actual Cargo graph with declared, checksum-pinned, offline inputs."""
load("@wasmono//:defs.bzl", "host_os")

def _cargo_impl(ctx):
    source = ctx.actions.symlinked_dir("source", ctx.attrs.srcs)
    output = ctx.actions.declare_output(("xtask.exe" if host_os() == "windows" else "xtask") if ctx.attrs.mode == "build" else ctx.attrs.mode + ".log")
    command = cmd_args("bash", ctx.attrs.script, ctx.attrs.sdk[DefaultInfo].default_outputs[0], source, ctx.attrs.mode, output.as_output())
    for name, dep in ctx.attrs.crates.items():
        command.add(name, dep[DefaultInfo].default_outputs[0], ctx.attrs.checksums[name])
    ctx.actions.run(command, category = "cargo_" + ctx.attrs.mode, local_only = True)
    providers = [DefaultInfo(default_output = output)]
    if ctx.attrs.mode == "build":
        providers.append(RunInfo(args = cmd_args(output)))
    return providers

cargo_workspace = rule(
    impl = _cargo_impl,
    attrs = {
        "srcs": attrs.dict(attrs.string(), attrs.source()),
        "sdk": attrs.exec_dep(default = "toolchains//rust:sdk"),
        "crates": attrs.dict(attrs.string(), attrs.dep()),
        "checksums": attrs.dict(attrs.string(), attrs.string()),
        "mode": attrs.enum(["build", "test", "lint", "fmt"]),
        "script": attrs.source(default = "//build-support:cargo.sh"),
    },
)
