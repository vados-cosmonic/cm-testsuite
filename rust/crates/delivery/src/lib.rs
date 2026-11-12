//! Apply reviewed exchange plans locally or publish guarded GitHub proposals.

mod distribute;
mod gather;
mod github;
mod local;
mod report;

pub use distribute::{Delivery, distribute};
pub use gather::{Gathered, Source, gather};
pub use github::{Publication, branch_for, publish};
pub use local::apply;
pub use report::{Failure, Report, report};
