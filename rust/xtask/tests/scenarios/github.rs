//! Live source → temporary catalog → recipient exchange, including updates and retirement.

use crate::{
    common::{UPDATED, WAST, manifest, policy, write},
    github_support::Sandbox,
};
use anyhow::{Result, ensure};
use cm_acquisition::{GitHub, acquire, load_catalog_files};
use cm_catalog::{Catalog, records};
use cm_exchange_plan::{Plan, import, receive};

#[test]
#[ignore = "creates private GitHub repositories; see MAINTAINERS.md#live-github-tests"]
fn github_exchange_round_trip() -> Result<()> {
    let mut sandbox = Sandbox::new()?;
    let source = sandbox.create("source")?;
    let hub = sandbox.create("catalog")?;
    let recipient = sandbox.create("recipient")?;
    let local = tempfile::tempdir()?;
    let catalog_root = local.path().join("catalog");
    let recipient_root = local.path().join("recipient");
    std::fs::create_dir_all(&catalog_root)?;
    let policy = policy("deletions = 'pull-request'");
    write(&recipient_root, ".cm-testsuite.toml", &policy);
    sandbox.commit_files(
        &recipient,
        &[
            (".cm-testsuite.toml", &policy),
            ("unrelated.txt", "keep recipient content"),
        ],
    )?;
    sandbox.commit_files(&hub, &[("unrelated.txt", "keep catalog content")])?;
    let export = manifest("file:///test.wast", "");
    let delivered = format!(
        "tests/shared/{}/nested/test.wast",
        source.source.strip_prefix("github:").unwrap()
    );

    // Exercise both a first import and an update through real acquisition and draft PRs.
    for content in [WAST, UPDATED] {
        sandbox.commit_files(
            &source,
            &[(".cm-testsuite.toml", &export), ("test.wast", content)],
        )?;
        let snapshot = acquire(
            &GitHub {
                api: &sandbox.api,
                source: &source.source,
            },
            &source.source,
            &source.branch,
            ".cm-testsuite.toml",
        )?;
        // Round-trip exchanged records as a consumer would, retaining validation.
        let snapshot = records::decode(&records::encode(&snapshot)?.into_bytes())?;
        let plan: Plan =
            records::decode(&records::encode(&import(&catalog_root, &[snapshot])?)?.into_bytes())?;
        sandbox.publish_and_merge(&hub, &plan)?;
        cm_delivery::apply(&catalog_root, &plan)?;
        let catalog = Catalog::load(&catalog_root)?;
        let plan = receive(
            &recipient_root,
            &recipient.source,
            ".cm-testsuite.toml",
            &catalog,
            &load_catalog_files(&catalog_root, &catalog)?,
        )?;
        sandbox.publish_and_merge(&recipient, &plan)?;
        cm_delivery::apply(&recipient_root, &plan)?;
        sandbox.assert_file(&recipient, &delivered, Some(content.as_bytes()))?;
        sandbox.assert_file(&source, "test.wast", Some(content.as_bytes()))?;
        ensure!(
            receive(
                &recipient_root,
                &recipient.source,
                ".cm-testsuite.toml",
                &catalog,
                &load_catalog_files(&catalog_root, &catalog)?
            )?
            .edits
            .is_empty(),
            "delivery must converge"
        );
    }

    // A committed opt-out retires the catalog entry and permits managed deletion.
    sandbox.commit_files(&source, &[(".cm-testsuite.toml", "schema = 1\n")])?;
    let snapshot = acquire(
        &GitHub {
            api: &sandbox.api,
            source: &source.source,
        },
        &source.source,
        &source.branch,
        ".cm-testsuite.toml",
    )?;
    let plan = import(&catalog_root, &[snapshot])?;
    sandbox.publish_and_merge(&hub, &plan)?;
    cm_delivery::apply(&catalog_root, &plan)?;
    let catalog = Catalog::load(&catalog_root)?;
    let plan = receive(
        &recipient_root,
        &recipient.source,
        ".cm-testsuite.toml",
        &catalog,
        &load_catalog_files(&catalog_root, &catalog)?,
    )?;
    sandbox.publish_and_merge(&recipient, &plan)?;
    sandbox.assert_file(&recipient, &delivered, None)?;
    sandbox.assert_file(&recipient, "unrelated.txt", Some(b"keep recipient content"))?;
    sandbox.assert_file(&hub, "unrelated.txt", Some(b"keep catalog content"))?;
    sandbox.assert_file(&source, "test.wast", Some(UPDATED.as_bytes()))?;
    sandbox.cleanup()?;
    Ok(())
}
