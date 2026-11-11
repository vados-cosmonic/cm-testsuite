"""Assemble the downloaded Rust components for Cargo, without system Rust."""
def _sdk_impl(ctx):
    host = ctx.attrs.distribution[DefaultInfo].default_outputs[0]
    out = ctx.actions.declare_output("sdk", dir = True)
    ctx.actions.run(
        cmd_args(ctx.attrs.bash, ctx.attrs.script, host, ctx.attrs.triple, ctx.attrs.version, out.as_output()),
        category = "rust_sdk",
        local_only = True,
        env = ctx.attrs.env,
    )
    return [DefaultInfo(default_output = out)]

rust_sdk = rule(
    impl = _sdk_impl,
    attrs = {
        "bash": attrs.string(),
        "env": attrs.dict(attrs.string(), attrs.string()),
        "distribution": attrs.dep(),
        "triple": attrs.string(),
        "version": attrs.string(),
        "script": attrs.source(),
    },
)
