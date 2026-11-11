//! Explicitly ignored live GitHub exchange; never runs in ordinary tests or CI.

mod common;

#[path = "scenarios/github_support.rs"]
mod github_support;

#[path = "scenarios/github.rs"]
mod github;
