//! GitHub access through Octocrab, with a mockable boundary for exchange logic.

mod client;
mod repository;
mod scoped;
mod sts;

pub use client::{Api, Client, HttpStatus, required};
pub use repository::{encode, read_file, repo_path, resolve, tree_sha};
pub use scoped::{Connect, Scoped};
pub use sts::{OctoSts, Session};
