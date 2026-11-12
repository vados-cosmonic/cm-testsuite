//! Fixtures shared by the workspace's tests. Depend on this crate only from
//! `[dev-dependencies]`: its helpers panic on failure instead of returning errors.

pub mod git;
pub mod github;
