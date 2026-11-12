# Participation

`participants.toml` lists the repositories the scheduled `gather` and `distribute`
runs look at, and the maintainers to mention when one of them fails.
Enrollment is approved during review.
Each repository's own manifest decides whether it shares tests, receives them, or both.
See the [workflow guide](../docs/src/workflow.md#gather-and-distribute) for the format and commands.

A repository that is not enrolled can still be imported or offered tests by hand
through the acquisition and planning commands or the manual workflow inputs.
See the [workflow guide](../docs/src/workflow.md#import) for usage.

`publishers.toml` lists the identifiers allowed to submit engine-result reports.
Membership controls admission; it does not verify a report's signature or origin.
See the [result-submission guide](../docs/src/results.md) for the report format
and admission process.
