//! Relocatable Cargo artifacts shared by Buck build and test targets.

pub mod export;
pub mod run;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Test binaries and rustdoc commands emitted by one workspace compilation.
#[derive(Default, Deserialize, Serialize)]
pub struct Bundle {
    pub tests: Vec<TestBinary>,
    pub docs: Vec<DocTest>,
}

/// A libtest executable, with its path relative to the artifact directory.
#[derive(Deserialize, Serialize)]
pub struct TestBinary {
    pub name: String,
    pub path: String,
    /// Owning package directory, relative to the bundle (not the target source).
    pub directory: String,
    pub integration: bool,
    pub live: bool,
}

/// Cargo's exact rustdoc invocation, with relocatable workspace and SDK paths.
#[derive(Deserialize, Serialize)]
pub struct DocTest {
    pub args: Vec<String>,
    pub directory: String,
    pub env: BTreeMap<String, String>,
}
