//! Isolated local repositories and real CLI invocations shared by E2E scenarios.

// Each integration-test executable uses only the helpers needed by its scenarios.
#![allow(dead_code)]

use cm_acquisition::Snapshot;
use cm_catalog::records;
use cm_exchange_plan::Plan;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub const SOURCE: &str = "github:example/source";
pub const RECIPIENT: &str = "github:example/recipient";
pub const WAST: &str = ";; Preserve these bytes: café\r\n(module)\r\n";
pub const UPDATED: &str = ";; Updated fixture\n(module (func))\n";
pub const DELIVERED: &str = "tests/shared/example/source/nested/test.wast";

/// Write a fixture without relying on platform-specific shell utilities.
pub fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// Return subprocess stdout, retaining useful diagnostics on a failed assertion.
pub fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Assert a diagnostic as well as failure, so unrelated errors cannot pass a test.
pub fn failure(output: Output, diagnostic: &str) {
    assert!(!output.status.success(), "command unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(diagnostic),
        "expected {diagnostic:?} in {stderr}"
    );
}

pub use cm_test_support::git::git;

/// Commit all fixture changes and return the immutable revision.
pub fn commit(root: &Path) -> String {
    cm_test_support::git::commit(root, "test: update fixture")
}

/// Build an offline command that cannot accidentally inherit publication credentials.
pub fn cli(root: &Path) -> Command {
    let mut command = Command::new(
        std::env::var_os("CM_TESTSUITE_XTASK")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_xtask").into()),
    );
    command
        .arg("--root")
        .arg(root)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_TOKEN")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

/// Encode an export fixture, allowing nested manifests and explicit file pins.
pub fn manifest(path: &str, extra: &str) -> String {
    format!(
        "schema = 1\n\n[[test]]\nname = 'nested/test'\npath = '{path}'\nsummary = 'Fixture intent'\ncovers = ['arithmetic']\nrequires = ['component-model']\n{extra}"
    )
}

/// Opt into a single receiving route; policy overrides belong before the route.
pub fn policy(extra: &str) -> String {
    format!(
        "schema = 1\n\n[receive]\nenabled = true\n{extra}\n\n[[receive.route]]\nname = 'shared'\ndestination = 'file:///tests/shared'\nsupports = ['component-model']\nallow_unknown = true\n"
    )
}

/// Three independent directories plus serialized plans, all removed after the test.
pub struct Exchange {
    pub temp: tempfile::TempDir,
    pub source: PathBuf,
    pub hub: PathBuf,
    pub recipient: PathBuf,
}

impl Default for Exchange {
    fn default() -> Self {
        Self::new()
    }
}

impl Exchange {
    /// Use spaces and Unicode in real filesystem roots to exercise argument handling.
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source project café");
        let hub = temp.path().join("catalog project");
        let recipient = temp.path().join("receiving project");
        for root in [&source, &hub, &recipient] {
            fs::create_dir_all(root).unwrap();
        }
        git(&source, &["init", "--quiet", "--initial-branch=main"]);
        write(&source, ".gitattributes", "* -text\n");
        write(
            &source,
            ".cm-testsuite.toml",
            manifest("file:///tests/input.wast", ""),
        );
        write(&source, "tests/input.wast", WAST);
        commit(&source);
        write(&recipient, ".cm-testsuite.toml", policy(""));
        write(&recipient, "README.md", "Unrelated recipient content\n");
        Self {
            temp,
            source,
            hub,
            recipient,
        }
    }

    /// Keep generated exchange records outside every participating repository.
    pub fn artifact(&self, name: &str) -> PathBuf {
        self.temp.path().join(name)
    }

    /// Acquire the committed source using the actual CLI and a serialized snapshot.
    pub fn acquire(&self, reference: &str, manifest: &str) -> Output {
        cli(&self.hub)
            .args([
                "acquire",
                "--source",
                SOURCE,
                "--reference",
                reference,
                "--manifest",
                manifest,
                "--local",
            ])
            .arg(&self.source)
            .arg("--out")
            .arg(self.artifact("snapshot.toml"))
            .output()
            .unwrap()
    }

    /// Read the generated snapshot through the production validation boundary.
    pub fn snapshot(&self) -> Snapshot {
        cm_acquisition::load_snapshot(&self.artifact("snapshot.toml")).unwrap()
    }

    /// Generate an import plan without mutating the catalog.
    pub fn plan_import(&self) -> Output {
        cli(&self.hub)
            .args(["plan-import", "--snapshot"])
            .arg(self.artifact("snapshot.toml"))
            .arg("--out")
            .arg(self.artifact("import.toml"))
            .output()
            .unwrap()
    }

    /// Apply a serialized proposal through the public CLI.
    pub fn apply(&self, root: &Path, name: &str) -> Output {
        cli(root)
            .args(["apply", "--plan"])
            .arg(self.artifact(name))
            .output()
            .unwrap()
    }

    /// Import a fresh committed source snapshot into the fixture catalog.
    pub fn import(&self) {
        success(self.acquire("main", ".cm-testsuite.toml"));
        success(self.plan_import());
        success(self.apply(&self.hub, "import.toml"));
        success(cli(&self.hub).arg("validate").output().unwrap());
    }

    /// Generate a recipient proposal against the catalog's checked-in bytes.
    pub fn plan_receive(&self) -> Output {
        cli(&self.recipient)
            .args(["plan-receive", "--recipient", RECIPIENT, "--catalog-root"])
            .arg(&self.hub)
            .arg("--out")
            .arg(self.artifact("receive.toml"))
            .output()
            .unwrap()
    }

    /// Deliver current catalog files and receipt into the recipient directory.
    pub fn receive(&self) {
        success(self.plan_receive());
        success(self.apply(&self.recipient, "receive.toml"));
    }

    /// Decode a CLI-produced proposal for assertions about edits and guards.
    pub fn plan(&self, name: &str) -> Plan {
        records::decode(&fs::read(self.artifact(name)).unwrap()).unwrap()
    }

    /// Commit new source bytes; callers decide when to import and deliver them.
    pub fn update(&self) {
        write(&self.source, "tests/input.wast", UPDATED);
        commit(&self.source);
    }
}
