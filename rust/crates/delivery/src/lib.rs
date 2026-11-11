//! Apply reviewed exchange plans locally or publish guarded GitHub proposals.

mod github;
mod local;

pub use github::{Publication, branch_for, publish};
pub use local::apply;
