//! Regression scenarios for executing Buck's relocated artifact bundles.

mod common;

#[path = "../src/build_artifacts/mod.rs"]
mod build_artifacts;

#[path = "scenarios/build_artifacts.rs"]
mod scenarios;
