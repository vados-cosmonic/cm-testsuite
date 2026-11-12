//! Gathering from and distributing to several enrolled repositories.

mod common;

use cm_acquisition::load_catalog_files;
use cm_catalog::{Catalog, Participant, Participants};
use cm_delivery::{
    Delivery, Failure, Publication, Report, Source, apply, branch_for, distribute, gather, publish,
    report,
};
use cm_exchange_plan::Plan;
use cm_test_support::github::FakeGitHubs;
use common::github::merge;
use serde_json::json;
use std::path::Path;

const MANIFEST: &str = ".cm-testsuite.toml";
const RECEIVING: &str = "schema=1\n[receive]\nenabled=true\n[[receive.route]]\nname='smoke'\n\
    destination='file:///shared'\nsupports=['component-model']\nallow_unknown=true\n";

/// Manifest sharing one test named `name` from `tests/<name>.wast`.
fn sharing(name: &str) -> String {
    format!(
        "schema=1\n[[test]]\nname='{name}'\npath='file:///tests/{name}.wast'\n\
         summary='Check {name}'\nrequires=['component-model']\ncovers=['smoke']\n"
    )
}

/// Give `repository` a manifest sharing the test `name` and its WAST file.
fn share(hub: &FakeGitHubs, repository: &str, name: &str) {
    hub.push(
        repository,
        &[
            (MANIFEST, &sharing(name)),
            (&format!("tests/{name}.wast"), ";; intent\n(module)\n"),
        ],
    );
}

fn enrolled(repositories: &[&str]) -> Vec<Participant> {
    let entries: String = repositories
        .iter()
        .map(|name| format!("[[participant]]\nrepository='github:{name}'\n"))
        .collect();
    Participants::parse(format!("schema=1\n{entries}").as_bytes())
        .unwrap()
        .participant
}

#[test]
fn failures_share_one_issue_that_closes_when_they_clear() {
    let hub = FakeGitHubs::new(&["example/catalog"]);
    let file = |failures: &[Failure]| {
        let maintainers = ["lead".to_owned()];
        report(
            &hub,
            "github:example/catalog",
            "distribute",
            failures,
            &maintainers,
            Some("https://example.invalid/run/1"),
        )
        .unwrap()
    };
    let failure = |repository: &str, error: &str| Failure {
        repository: repository.into(),
        error: error.into(),
        contacts: vec!["owner".into()],
    };

    // Nothing is filed for a clean run.
    assert_eq!(file(&[]), Report::Clear);
    assert_eq!(hub.writes(), 0);

    // Repeated failing runs rewrite one issue instead of opening more.
    let first = [failure(
        "github:example/r1",
        "missing | manifest\n@everyone",
    )];
    assert!(matches!(file(&first), Report::Filed(_)));
    let both = [first[0].clone(), failure("github:example/r2", "HTTP 403")];
    assert!(matches!(file(&both), Report::Updated(_)));
    {
        let state = hub.repository("example/catalog").state();
        assert_eq!(state.issues.len(), 1);
        assert_eq!(
            state.issues[0]["title"],
            "Scheduled distribute run has failures"
        );
        let body = state.issues[0]["body"].as_str().unwrap();
        assert!(
            body.contains(
                "| `github:example/r1` | missing \\| manifest @\u{200b}everyone | @owner |"
            )
        );
        assert!(body.contains("| `github:example/r2` | HTTP 403 | @owner |"));
        assert!(body.contains("cc @lead"));
        assert!(body.contains("registry/participants.toml"));
        assert!(body.contains("https://example.invalid/run/1"));
    }

    // Another task keeps its own issue, and a clean run closes only its own.
    assert!(matches!(
        report(&hub, "github:example/catalog", "gather", &first, &[], None).unwrap(),
        Report::Filed(_)
    ));
    assert!(matches!(file(&[]), Report::Resolved(_)));
    let state = hub.repository("example/catalog").state();
    assert_eq!(state.issues[0]["state"], "closed");
    assert_eq!(state.issues[1]["state"], "open");
    drop(state);

    // A failure after that opens a fresh issue, notifying again.
    assert!(matches!(file(&first), Report::Filed(_)));
    assert_eq!(hub.repository("example/catalog").state().issues.len(), 3);
}

/// The import of one enrolled repository against the catalog in `root`.
fn plan(hub: &FakeGitHubs, root: &Path, name: &str) -> Plan {
    let source = gather(hub, root, &enrolled(&[name])[0]).unwrap();
    source.plan().clone()
}

/// Gather the enrolled repositories and apply each import to the catalog in `root`.
fn import(hub: &FakeGitHubs, root: &Path, names: &[&str]) {
    for name in names {
        apply(root, &plan(hub, root, name)).unwrap();
    }
}

/// Gather each repository and publish its import, as a publishing run does.
/// A repository that fails leaves its pull request alone.
fn round(hub: &FakeGitHubs, root: &Path, names: &[&str]) -> Vec<Option<Publication>> {
    let run = |name: &&str| {
        let source = gather(hub, root, &enrolled(&[name])[0]).ok()?;
        Some(publish(hub, CATALOG, source.plan()).unwrap())
    };
    names.iter().map(run).collect()
}

/// The catalog repository's pull requests as (branch, state), in creation order.
fn proposals(hub: &FakeGitHubs) -> Vec<(String, String)> {
    let state = hub.repository("example/catalog").state();
    let text = |value: &serde_json::Value| value.as_str().unwrap().to_owned();
    let prs = state.prs.iter();
    prs.map(|pr| (text(&pr["head"]), text(&pr["state"])))
        .collect()
}

/// The commit a branch of the catalog repository points at.
fn head(hub: &FakeGitHubs, branch: &str) -> String {
    hub.repository("example/catalog").state().refs[branch].clone()
}

const CATALOG: &str = "github:example/catalog";
const A_BRANCH: &str = "cm-testsuite/sources/example/a";
const B_BRANCH: &str = "cm-testsuite/sources/example/b";

#[test]
fn gathers_each_source_into_its_own_import_and_isolates_failures() {
    let names = [
        "example/a",
        "example/b",
        "example/receiver",
        "example/broken",
    ];
    let hub = FakeGitHubs::new(&names);
    share(&hub, "example/a", "one");
    share(&hub, "example/b", "two");
    hub.push("example/receiver", &[(MANIFEST, RECEIVING)]);
    let participants = enrolled(&names);
    let catalog = tempfile::tempdir().unwrap();
    let found = |name: usize| gather(&hub, catalog.path(), &participants[name]);

    for (name, subject) in [(0, "github:example/a"), (1, "github:example/b")] {
        let Source::Acquired { tests, plan, .. } = found(name).unwrap() else {
            panic!("expected a snapshot of {subject}")
        };
        assert_eq!((tests, plan.changes().added), (1, 1));
        assert_eq!(plan.subject, subject);
        assert_eq!(
            plan.title(),
            format!("feat(corpus): import tests from {}", &subject[7..])
        );
    }
    // A repository that only receives is not imported, and a failure is its own.
    let Source::NoTests(nothing) = found(2).unwrap() else {
        panic!("expected no tests")
    };
    assert!(nothing.edits.is_empty());
    assert!(found(3).is_err());
    assert_eq!(hub.writes(), 0);

    // A repeat after applying finds nothing, and a failing source retires nothing.
    import(&hub, catalog.path(), &names[..2]);
    for name in &names[..2] {
        assert!(plan(&hub, catalog.path(), name).edits.is_empty());
    }
    let loaded = Catalog::load(catalog.path()).unwrap();
    assert_eq!(loaded.tests.len(), 2);
    assert!(!loaded.sources.contains_key("github:example/receiver"));

    // A commit that touches no shared test proposes nothing, although the
    // source's head moved.
    hub.push("example/a", &[("README.md", "unrelated\n")]);
    assert!(matches!(
        found(0).unwrap(),
        Source::Acquired { tests: 1, plan, .. } if plan.edits.is_empty()
    ));

    // An edit to a shared test is proposed.
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; intent\n(component)\n")],
    );
    assert_eq!(plan(&hub, catalog.path(), "example/a").changes().updated, 1);

    // A source that drops its tests retires them instead of being skipped.
    hub.push("example/b", &[(MANIFEST, "schema=1\n")]);
    let retiring = plan(&hub, catalog.path(), "example/b");
    assert_eq!(retiring.changes().removed, 1);
}

#[test]
fn every_source_has_one_pull_request_on_its_own_branch() {
    let hub = FakeGitHubs::new(&["example/a", "example/b", "example/catalog"]);
    share(&hub, "example/a", "one");
    share(&hub, "example/b", "two");
    let catalog = tempfile::tempdir().unwrap();
    let names = ["example/a", "example/b"];
    let open = |branch: &str| (branch.to_owned(), "open".to_owned());

    // The branch depends on the source alone, not on what else the run covers.
    round(&hub, catalog.path(), &names[..1]);
    round(&hub, catalog.path(), &names);
    assert_eq!(proposals(&hub), [open(A_BRANCH), open(B_BRANCH)]);
    {
        let state = hub.repository("example/catalog").state();
        for (pr, name) in state.prs.iter().zip(["a/one", "b/two"]) {
            let source = name.split('/').next().unwrap();
            assert_eq!(
                pr["title"],
                format!("feat(corpus): import tests from example/{source}")
            );
            assert_eq!(pr["draft"], true);
            let body = pr["body"].as_str().unwrap();
            assert!(body.contains("into the catalog: 1 added."), "{body}");
            assert!(
                body.contains(&format!("| Add | `github:example/{name}` |")),
                "{body}"
            );
        }
    }

    // A repeated run changes nothing.
    let commits = hub.repository("example/catalog").state().commits.len();
    let writes = hub.writes();
    round(&hub, catalog.path(), &names);
    assert_eq!(
        hub.repository("example/catalog").state().commits.len(),
        commits
    );
    assert_eq!(proposals(&hub).len(), 2);

    // A source that moves on updates its own pull request in place.
    let (a, b) = (head(&hub, A_BRANCH), head(&hub, B_BRANCH));
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; intent\n(component)\n")],
    );
    round(&hub, catalog.path(), &names);
    assert_eq!(proposals(&hub), [open(A_BRANCH), open(B_BRANCH)]);
    assert_ne!(head(&hub, A_BRANCH), a);
    assert_eq!(head(&hub, B_BRANCH), b);
    assert!(hub.writes() > writes);

    // An import of the same source planned by hand lands in that pull request too.
    let manual = cm_acquisition::acquire(
        &cm_acquisition::GitHub {
            api: &hub,
            source: "github:example/a",
        },
        "github:example/a",
        "main",
        MANIFEST,
    )
    .unwrap();
    let manual = cm_exchange_plan::import(catalog.path(), &[manual]).unwrap();
    assert_eq!(branch_for(&manual), A_BRANCH);
    let a = head(&hub, A_BRANCH);
    publish(&hub, CATALOG, &manual).unwrap();
    assert_eq!(proposals(&hub).len(), 2);
    assert_eq!(head(&hub, A_BRANCH), a);
}

#[test]
fn a_source_that_fails_keeps_its_pull_request_while_the_others_update() {
    let hub = FakeGitHubs::new(&["example/a", "example/b", "example/catalog"]);
    share(&hub, "example/a", "one");
    share(&hub, "example/b", "two");
    let catalog = tempfile::tempdir().unwrap();
    let names = ["example/a", "example/b"];
    round(&hub, catalog.path(), &names);
    let (a, b) = (head(&hub, A_BRANCH), head(&hub, B_BRANCH));
    let body = |number: usize| {
        let state = hub.repository("example/catalog").state();
        state.prs[number]["body"].clone()
    };
    let described = body(1);

    // One source can no longer be read while the other moves on.
    hub.push("example/b", &[(MANIFEST, "schema=2\n")]);
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; intent\n(component)\n")],
    );
    let outcome = round(&hub, catalog.path(), &names);
    assert!(matches!(outcome[0], Some(Publication::PullRequest(_))));
    assert_eq!(outcome[1], None);
    assert_ne!(head(&hub, A_BRANCH), a);
    assert_eq!(head(&hub, B_BRANCH), b);
    assert_eq!(body(1), described);
    assert!(proposals(&hub).iter().all(|(_, state)| state == "open"));
}

#[test]
fn a_source_with_nothing_new_withdraws_only_its_own_pull_request() {
    let hub = FakeGitHubs::new(&["example/a", "example/b", "example/catalog"]);
    share(&hub, "example/a", "one");
    share(&hub, "example/b", "two");
    let catalog = tempfile::tempdir().unwrap();
    let names = ["example/a", "example/b"];
    let state = |branch: &str, state: &str| (branch.to_owned(), state.to_owned());
    round(&hub, catalog.path(), &names);

    // A source that stops sharing before its first import lands has nothing to propose.
    hub.push("example/a", &[(MANIFEST, "schema=1\n")]);
    let outcome = round(&hub, catalog.path(), &names);
    assert!(matches!(outcome[0], Some(Publication::Closed(_))));
    assert!(matches!(outcome[1], Some(Publication::PullRequest(_))));
    assert_eq!(
        proposals(&hub),
        [state(A_BRANCH, "closed"), state(B_BRANCH, "open")]
    );
    assert_eq!(
        round(&hub, catalog.path(), &names[..1]),
        [Some(Publication::NoChanges)]
    );

    // Sharing again opens a new pull request on the same branch.
    share(&hub, "example/a", "one");
    let first = plan(&hub, catalog.path(), "example/a");
    round(&hub, catalog.path(), &names);
    assert_eq!(proposals(&hub)[2], state(A_BRANCH, "open"));

    // Once that import has landed, a change that is taken back is withdrawn too.
    merge(hub.repository("example/catalog"), &first);
    apply(catalog.path(), &first).unwrap();
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; intent\n(component)\n")],
    );
    round(&hub, catalog.path(), &names);
    assert_eq!(proposals(&hub)[3], state(A_BRANCH, "open"));
    hub.push("example/a", &[("tests/one.wast", ";; intent\n(module)\n")]);
    let outcome = round(&hub, catalog.path(), &names);
    assert!(matches!(outcome[0], Some(Publication::Closed(_))));
    assert_eq!(
        proposals(&hub),
        [
            state(A_BRANCH, "closed"),
            state(B_BRANCH, "open"),
            state(A_BRANCH, "closed"),
            state(A_BRANCH, "closed"),
        ]
    );
}

#[test]
fn per_source_pull_requests_merge_in_either_order() {
    let names = ["example/a", "example/b"];
    for order in [[0, 1], [1, 0]] {
        let hub = FakeGitHubs::new(&["example/a", "example/b", "example/catalog"]);
        share(&hub, "example/a", "one");
        share(&hub, "example/b", "two");
        let catalog = tempfile::tempdir().unwrap();
        let remote = hub.repository("example/catalog");
        let plans = names.map(|name| plan(&hub, catalog.path(), name));
        round(&hub, catalog.path(), &names);
        let (first, second) = (order[0], order[1]);

        // One lands. The other, planned against the catalog before that, is
        // neither stale nor in conflict: it touches no file the first one did.
        merge(remote, &plans[first]);
        let kept = publish(&hub, CATALOG, &plans[second]).unwrap();
        assert!(matches!(kept, Publication::PullRequest(_)), "{kept:?}");

        // The next run, from a checkout of the new default branch, agrees.
        apply(catalog.path(), &plans[first]).unwrap();
        let outcome = round(&hub, catalog.path(), &names);
        assert_eq!(outcome[first], Some(Publication::NoChanges));
        assert!(matches!(outcome[second], Some(Publication::PullRequest(_))));
        assert_eq!(plan(&hub, catalog.path(), names[second]), plans[second]);

        // The other lands too, and nothing is left to propose.
        merge(remote, &plans[second]);
        apply(catalog.path(), &plans[second]).unwrap();
        assert_eq!(
            round(&hub, catalog.path(), &names),
            [Some(Publication::NoChanges), Some(Publication::NoChanges)]
        );
        assert_eq!(proposals(&hub).len(), 2);

        // The default branch holds exactly the catalog the run's checkout does.
        let landed = tempfile::tempdir().unwrap();
        {
            let state = remote.state();
            for (path, text) in state.files_at(&state.refs["main"]) {
                let file = landed.path().join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, text).unwrap();
            }
        }
        let loaded = Catalog::load(landed.path()).unwrap();
        assert_eq!(loaded, Catalog::load(catalog.path()).unwrap());
        assert_eq!(loaded.tests.len(), 2);
        assert_eq!(load_catalog_files(landed.path(), &loaded).unwrap().len(), 2);
    }
}

#[test]
fn distributes_one_pull_request_per_receiving_repository() {
    let hub = FakeGitHubs::new(&["example/a", "example/r1", "example/r2", "example/plain"]);
    share(&hub, "example/a", "one");
    hub.push("example/r1", &[(MANIFEST, RECEIVING)]);
    hub.push("example/r2", &[(MANIFEST, RECEIVING)]);
    hub.push("example/plain", &[(MANIFEST, "schema=1\n")]);
    let root = tempfile::tempdir().unwrap();
    import(&hub, root.path(), &["example/a"]);
    let run = |name: &str, publish: bool| {
        let catalog = Catalog::load(root.path()).unwrap();
        let files = load_catalog_files(root.path(), &catalog).unwrap();
        distribute(&hub, &enrolled(&[name])[0], &catalog, &files, publish)
    };

    // Checking never writes, and a repository that has not opted in is left alone.
    assert!(matches!(
        run("example/plain", true).unwrap(),
        Delivery::NotReceiving
    ));
    let Delivery::Planned(plan) = run("example/r1", false).unwrap() else {
        panic!("expected a dry run")
    };
    assert_eq!(plan.changes().added, 1);
    assert_eq!(hub.writes(), 0);
    assert!(run("example/missing", true).is_err());

    // Each receiving repository gets its own draft, and a repeat changes nothing.
    for name in ["example/r1", "example/r2", "example/r1"] {
        assert!(matches!(
            run(name, true).unwrap(),
            Delivery::Published(_, Publication::PullRequest(_))
        ));
    }
    let commits = hub.repository("example/r1").state().commits.len();
    run("example/r1", true).unwrap();
    assert_eq!(hub.repository("example/r1").state().commits.len(), commits);
    for name in ["example/r1", "example/r2"] {
        let state = hub.repository(name).state();
        assert_eq!(state.prs.len(), 1);
        assert_eq!(
            state.prs[0]["title"],
            "Update tests received from cm-testsuite"
        );
        let body = state.prs[0]["body"].as_str().unwrap();
        assert!(body.contains("| Add | `github:example/a/one` |"), "{body}");
    }

    // A later catalog change lands in the same pull request.
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; revised intent\n(component)\n")],
    );
    import(&hub, root.path(), &["example/a"]);
    run("example/r1", true).unwrap();
    {
        let state = hub.repository("example/r1").state();
        assert_eq!(state.prs.len(), 1);
        let head = &state.refs[&branch_for(&plan)];
        assert_eq!(
            state.files_at(head)["shared/example/a/one.wast"],
            ";; revised intent\n(component)\n"
        );
    }

    // Once the recipient merges it, there is nothing left to propose.
    {
        let mut state = hub.repository("example/r1").state_mut();
        let head = state.refs[&branch_for(&plan)].clone();
        state.refs.insert("main".into(), head);
        state.prs[0]["state"] = json!("closed");
        state.prs[0]["merged_at"] = json!("2026-11-12T00:00:00Z");
    }
    assert!(matches!(
        run("example/r1", true).unwrap(),
        Delivery::Published(_, Publication::NoChanges)
    ));
    assert_eq!(hub.repository("example/r1").state().prs.len(), 1);
}
