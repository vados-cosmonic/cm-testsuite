"""Keep compilation local while sharing action outputs through an REAPI cache."""

load("@prelude//cfg/exec_platform:marker.bzl", "get_exec_platform_marker")

def _cached_execution_platform_impl(ctx):
    constraints = dict(ctx.attrs.cpu_configuration[ConfigurationInfo].constraints)
    constraints.update(ctx.attrs.os_configuration[ConfigurationInfo].constraints)
    configuration = ConfigurationInfo(constraints = constraints, values = {})
    name = ctx.label.raw_target()
    platform = ExecutionPlatformInfo(
        label = name,
        configuration = configuration,
        executor_config = CommandExecutorConfig(
            local_enabled = True,
            remote_enabled = False,
            remote_cache_enabled = True,
            allow_cache_uploads = True,
            use_windows_path_separators = host_info().os.is_windows,
        ),
    )
    return [
        DefaultInfo(),
        platform,
        PlatformInfo(label = str(name), configuration = configuration),
        ExecutionPlatformRegistrationInfo(
            platforms = [platform],
            exec_marker_constraint = get_exec_platform_marker(),
        ),
    ]

cached_execution_platform = rule(
    impl = _cached_execution_platform_impl,
    attrs = {
        "cpu_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "os_configuration": attrs.dep(providers = [ConfigurationInfo]),
    },
)
