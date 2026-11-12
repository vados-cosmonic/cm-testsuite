//! Shared repository fixtures and publication plans for delivery tests. The
//! in-memory GitHub repository lives in `cm-test-support`.
// Each integration-test binary uses only its own subset of these helpers.
#![allow(dead_code)]

pub mod exchange {
    use anyhow::Result;
    use cm_acquisition::{Repository, Snapshot, acquire, load_catalog_files};
    use cm_catalog::Catalog;
    use cm_delivery::apply;
    use cm_exchange_plan::{import, receive};
    use std::{collections::BTreeMap, fs, path::Path};
    /// Repository fixture that fails before any snapshot can be acquired.
    pub struct Gone;
    impl Repository for Gone {
        fn resolve(&self, _: &str) -> Result<String> {
            anyhow::bail!("network unavailable")
        }
        fn read(&self, _: &str, _: &str) -> Result<Vec<u8>> {
            unreachable!()
        }
    }

    pub struct Source {
        pub manifest: String,
        pub files: BTreeMap<String, String>,
    }
    impl Repository for Source {
        fn resolve(&self, r: &str) -> Result<String> {
            Ok(if r == "old" || r == "a".repeat(40) {
                "a".repeat(40)
            } else {
                "b".repeat(40)
            })
        }
        fn read(&self, r: &str, p: &str) -> Result<Vec<u8>> {
            if p == ".cm-testsuite.toml" {
                return Ok(self.manifest.as_bytes().to_vec());
            }
            if r == "a".repeat(40) {
                return Ok(b";; Old revision intent\n(module)\n".to_vec());
            }
            self.files
                .get(p)
                .map(|x| x.as_bytes().to_vec())
                .ok_or_else(|| anyhow::anyhow!("missing file"))
        }
    }
    /// Acquire one committed fixture test with stable provenance.
    pub fn snapshot(name: &str, path: &str, text: &str) -> Snapshot {
        let source = Source {
            manifest: format!(
                "schema=1\n\n[[test]]\nname='stable-name'\npath='file:///{path}'\n\
                 requires=['component-model']\ncovers=['smoke']\n",
            ),
            files: BTreeMap::from([(path.into(), text.into())]),
        };
        acquire(
            &source,
            &format!("github:example/{name}"),
            "main",
            ".cm-testsuite.toml",
        )
        .unwrap()
    }
    /// Import a fixture snapshot and return the resulting catalog.
    pub fn setup(root: &Path, snapshot: &Snapshot) -> Catalog {
        apply(root, &import(root, std::slice::from_ref(snapshot)).unwrap()).unwrap();
        Catalog::load(root).unwrap()
    }
    /// Write a receiving policy with scenario-specific overrides.
    pub fn policy(root: &Path, extra: &str) {
        fs::write(root.join(".cm-testsuite.toml"),format!("schema=1\n[receive]\nenabled=true\n{extra}\n[[receive.route]]\nname='smoke'\ndestination='file:///tests/shared'\nsupports=['component-model']\nallow_unknown=true\n")).unwrap();
    }
    /// Plan receiving from the current fixture catalog.
    pub fn received(root: &Path, catalog_root: &Path) -> cm_exchange_plan::Plan {
        let c = Catalog::load(catalog_root).unwrap();
        receive(
            root,
            "github:example/recipient",
            ".cm-testsuite.toml",
            &c,
            &load_catalog_files(catalog_root, &c).unwrap(),
        )
        .unwrap()
    }
}

pub mod github {
    use cm_exchange_plan::{Edit, Kind, Plan};
    use std::collections::{BTreeMap, BTreeSet};

    /// Create the standard managed proposal used by publication scenarios.
    pub fn plan(text: &str) -> Plan {
        Plan {
            schema: 1,
            kind: Kind::Import,
            subject: "github:example/source".into(),
            edits: vec![Edit {
                path: "corpus/wast/example/source/a.wast".into(),
                before: None,
                after: Some(text.into()),
            }],
            guards: vec![],
            warnings: vec![],
            exclusions: BTreeSet::new(),
            provenance: BTreeMap::new(),
        }
    }
}
