//! Apply reviewed exchange plans locally or publish guarded GitHub proposals.

mod distribute;
mod gather;
mod github;
mod local;
mod report;

pub use distribute::{Delivery, distribute};
pub use gather::{Gathered, Source, gather};
pub use github::{GATHER_BRANCH, Publication, branch_for, publish, publish_on};
pub use local::apply;
pub use report::{Failure, Report, report};
