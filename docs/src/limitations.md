# Limitations and next steps

This draft establishes a working local flow and reusable APIs, not a production service.

- Structural parsing does not execute assertions or establish their semantic correctness. Feature inference remains incomplete, including quoted/binary and deliberately malformed payloads.
- Stable test-name and repository aliases are not implemented. Preserve identities during source-path moves; review identity changes manually.
- Discovery is explicit per source; scheduled registry polling and automatic post-merge fan-out are future work. The CLI supports an alternate manifest location, but no persistent source-registry schema is implemented yet.
- Decline suppression is keyed by the complete plan digest. A different proposal can include the same test version; use permanent ID exclusions to stop future offers. Per-test declined-version state is future work.
- Publication supports same-repository draft PRs with at most 200 file edits. Fork delivery, large-tree optimizations, retries/backoff, and a live cross-repository pilot remain open.
- GitHub reads traverse regular-file trees and reject truncation. The draft caps source files at 8 MiB and uses the client's response limit; larger sources need a deliberate size policy.
- Local application has preflight checks, staging, and rollback for ordinary errors. It is not crash-atomic across files and cannot coordinate unrelated editors. Prefer clean worktrees.
- Linux x86_64 is the validated build host. Rust and dependency inputs are pinned, but the host linker/C compiler and operating system are not fully hermetic. Buck currently runs Cargo's actual graph rather than exposing per-crate native Buck targets.
- Report publisher approval is review-based, not signature verification. Historical reports are retained, but the site shows only the current catalog; retention and historical browsing remain future work.
- Test licensing is a review responsibility; automated policy checks are not implemented.

Other languages, generated WAST, separate Wasm binaries, non-Git transports, and execution runners remain outside the initial scope.
