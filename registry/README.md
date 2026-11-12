# Participation

`participants/` holds one file per repository the scheduled `gather` and `distribute` runs look at,
with the maintainers to mention when it fails;
see its [README](participants/README.md).
`participants.toml` holds the settings shared by all of them,
including the maintainers mentioned on every failure report.
Enrollment is approved during review.
Each repository's own manifest decides whether it shares tests, receives them, or both.
See the [workflow guide](../docs/src/workflow.md#gather-and-distribute) for the format and commands.
A repository that receives tests must also commit the
[Octo STS trust policy](../docs/src/workflow.md#credentials-in-github-actions)
that lets the scheduled run open its pull request.

A repository that is not enrolled can still be imported or offered tests by hand
through the acquisition and planning commands or the manual workflow inputs.
See the [workflow guide](../docs/src/workflow.md#import) for usage.

`publishers.toml` lists the identifiers allowed to submit engine-result reports.
Membership controls admission; it does not verify a report's signature or origin.
See the [result-submission guide](../docs/src/results.md) for the report format
and admission process.
