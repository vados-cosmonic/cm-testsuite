//! GitHub access through Octocrab, with a mockable boundary for exchange logic.

mod client;
mod repository;

pub use client::{Api, Client, HttpStatus, required};
pub use repository::{encode, read_file, repo_path, resolve, tree_sha};
