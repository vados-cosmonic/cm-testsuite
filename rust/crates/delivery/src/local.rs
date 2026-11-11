//! Apply reviewed edits with preflight validation and rollback on ordinary errors.

use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use cm_exchange_plan::{Plan, hash_at, read};
use std::{fs, path::Path};

/// Validate every precondition before writes; roll back completed writes on an I/O error.
/// The repository lock coordinates this tool, not arbitrary external editors.
pub fn apply(root: &Path, plan: &Plan) -> Result<usize> {
    plan.validate()?;
    let lock_path = checked_path(root, ".cm-testsuite-write-lock")?;
    // Serialize this tool's writers before reading any file baselines.
    let lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .context("another apply may be active; inspect .cm-testsuite-write-lock before removing a stale lock")?;
    struct Lock(std::path::PathBuf, fs::File);
    impl Drop for Lock {
        fn drop(&mut self) {
            let _ = self.1.sync_all();
            let _ = fs::remove_file(&self.0);
        }
    }
    let _lock = Lock(lock_path, lock);
    let desired = |e: &cm_exchange_plan::Edit| e.after.as_ref().map(|s| sha256(s.as_bytes()));
    let already = plan
        .edits
        .iter()
        .map(|e| Ok(hash_at(root, &e.path)? == desired(e)))
        .collect::<Result<Vec<_>>>()?;
    if already.iter().all(|x| *x) {
        return Ok(0);
    }
    for guard in &plan.guards {
        ensure!(
            hash_at(root, &guard.path)? == guard.sha256,
            "stale plan: {} changed",
            guard.path
        );
    }

    // Stage every replacement and retain original bytes before mutating the tree.
    let scratch = tempfile::Builder::new()
        .prefix(".cm-testsuite-stage-")
        .tempdir_in(root)?;
    let mut originals = vec![];
    for (i, e) in plan.edits.iter().enumerate() {
        ensure!(
            hash_at(root, &e.path)? == e.before,
            "stale plan or local edits: {}",
            e.path
        );
        originals.push(read(root, &e.path)?);
        if let Some(text) = &e.after {
            fs::write(scratch.path().join(i.to_string()), text)?;
        }
    }

    // Recheck each baseline immediately before the corresponding mutation.
    let mut completed = 0;
    let result = (|| -> Result<()> {
        for (i, e) in plan.edits.iter().enumerate() {
            let path = checked_path(root, &e.path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            ensure!(
                hash_at(root, &e.path)? == e.before,
                "file changed during apply: {}",
                e.path
            );
            if e.after.is_some() {
                fs::rename(scratch.path().join(i.to_string()), path)?;
            } else {
                fs::remove_file(path)?;
            }
            completed = i + 1;
        }
        Ok(())
    })();

    // Reverse completed writes, refusing to overwrite intervening external edits.
    if let Err(error) = result {
        let mut failures = vec![];
        for (i, original) in originals.iter().enumerate().take(completed).rev() {
            let e = &plan.edits[i];
            let rollback = (|| -> Result<()> {
                ensure!(
                    hash_at(root, &e.path)? == desired(e),
                    "external edit during rollback"
                );
                let path = checked_path(root, &e.path)?;
                match original {
                    Some(bytes) => fs::write(path, bytes)?,
                    None => fs::remove_file(path)?,
                };
                Ok(())
            })();
            if let Err(e) = rollback {
                failures.push(e.to_string());
            }
        }
        ensure!(
            failures.is_empty(),
            "apply failed ({error}); rollback needs manual recovery: {failures:?}"
        );
        return Err(error);
    }
    Ok(plan.edits.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_exchange_plan::{Edit, Guard, Kind};
    use std::collections::BTreeSet;
    fn plan() -> Plan {
        Plan {
            schema: 1,
            kind: Kind::Import,
            subject: "github:example/source".into(),
            edits: vec![Edit {
                path: "corpus/wast/a.wast".into(),
                before: None,
                after: Some("(module)".into()),
            }],
            guards: vec![],
            warnings: vec![],
            exclusions: BTreeSet::new(),
            provenance: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn apply_is_idempotent_and_preserves_unmanaged_files() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("README"), "local").unwrap();
        let p = plan();
        assert_eq!(apply(d.path(), &p).unwrap(), 1);
        assert_eq!(apply(d.path(), &p).unwrap(), 0);
        assert_eq!(
            fs::read_to_string(d.path().join("README")).unwrap(),
            "local"
        );
    }

    #[test]
    fn preflights_entire_plan_before_any_write() {
        let d = tempfile::tempdir().unwrap();
        let mut p = plan();
        p.edits.push(Edit {
            path: "corpus/wast/missing.wast".into(),
            before: Some("a".repeat(64)),
            after: None,
        });
        assert!(apply(d.path(), &p).is_err());
        assert!(!d.path().join("corpus/wast/a.wast").exists());
        assert!(!d.path().join(".cm-testsuite-write-lock").exists());
    }

    #[test]
    fn stale_policy_blocks_application() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("policy.toml"), "new").unwrap();
        let mut p = plan();
        p.guards.push(Guard {
            path: "policy.toml".into(),
            sha256: Some(sha256(b"old")),
        });
        assert!(apply(d.path(), &p).is_err());
    }

    #[test]
    fn collisions_are_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("corpus/wast")).unwrap();
        fs::write(d.path().join("corpus/wast/a.wast"), "my edit").unwrap();
        assert!(apply(d.path(), &plan()).is_err());
    }
}
