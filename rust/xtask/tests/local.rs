//! Offline end-to-end scenarios using real Git repositories and the public CLI.

mod common;

#[path = "scenarios/acquisition.rs"]
mod acquisition;

#[path = "scenarios/exchange.rs"]
mod exchange;

#[path = "scenarios/filesystem.rs"]
mod filesystem;
