//! Gathering from and distributing to several enrolled repositories.

use cm_acquisition::load_catalog_files;
use cm_catalog::{Catalog, Participant, Participants};
use cm_delivery::{
    Delivery, Failure, GATHER_BRANCH, Offer, Publication, Report, Review, Source, apply,
    branch_for, distribute, gather, publish, publish_on, report,
};
use cm_test_support::github::FakeGitHubs;
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

/// Gather the enrolled repositories and apply the import to the catalog in `root`.
fn import(hub: &FakeGitHubs, root: &Path, participants: &[Participant]) {
    let plan = gather(hub, root, participants).unwrap().plan.unwrap();
    apply(root, &plan).unwrap();
}

#[test]
fn gathers_every_source_into_one_import_and_isolates_failures() {
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

    let gathered = gather(&hub, catalog.path(), &participants).unwrap();
    assert_eq!(hub.writes(), 0);
    let found: Vec<_> = gathered.sources.iter().map(|(_, source)| source).collect();
    assert!(matches!(found[0], Source::Acquired { tests: 1, .. }));
    assert!(matches!(found[1], Source::Acquired { tests: 1, .. }));
    assert!(matches!(found[2], Source::NoTests));
    assert!(matches!(found[3], Source::Failed(_)));
    let plan = gathered.plan.unwrap();
    assert_eq!(plan.changes().added, 2);
    assert_eq!(plan.subject, "github:example/a, github:example/b");

    // A repeat after applying finds nothing, and a failing source retires nothing.
    apply(catalog.path(), &plan).unwrap();
    let repeat = gather(&hub, catalog.path(), &participants).unwrap();
    assert!(repeat.plan.unwrap().edits.is_empty());
    assert_eq!(Catalog::load(catalog.path()).unwrap().tests.len(), 2);

    // A commit that touches no shared test proposes nothing, although the
    // source's head moved.
    hub.push("example/a", &[("README.md", "unrelated\n")]);
    let unrelated = gather(&hub, catalog.path(), &participants).unwrap();
    assert!(matches!(
        unrelated.sources[0].1,
        Source::Acquired { tests: 1, .. }
    ));
    assert!(unrelated.plan.unwrap().edits.is_empty());

    // An edit to a shared test is proposed.
    hub.push(
        "example/a",
        &[("tests/one.wast", ";; intent\n(component)\n")],
    );
    let edited = gather(&hub, catalog.path(), &participants).unwrap();
    assert_eq!(edited.plan.unwrap().changes().updated, 1);

    // A source that drops its tests retires them instead of being skipped.
    hub.push("example/b", &[(MANIFEST, "schema=1\n")]);
    let retiring = gather(&hub, catalog.path(), &participants).unwrap();
    assert_eq!(retiring.plan.unwrap().changes().removed, 1);
}

#[test]
fn whole_registry_imports_share_one_pull_request_that_partial_plans_leave_alone() {
    let hub = FakeGitHubs::new(&["example/a", "example/b", "example/catalog"]);
    share(&hub, "example/a", "one");
    share(&hub, "example/b", "two");
    let catalog = tempfile::tempdir().unwrap();
    let plan = |names: &[&str]| {
        gather(&hub, catalog.path(), &enrolled(names))
            .unwrap()
            .plan
            .unwrap()
    };

    // Whole-registry runs replace each other in one pull request as the registry grows.
    let first = plan(&["example/a"]);
    let both = plan(&["example/a", "example/b"]);
    let ready = Offer {
        review: Review::Ready,
        recreate: false,
    };
    publish_on(&hub, "github:example/catalog", &first, GATHER_BRANCH, ready).unwrap();
    publish_on(&hub, "github:example/catalog", &both, GATHER_BRANCH, ready).unwrap();
    assert_eq!(hub.repository("example/catalog").state().prs.len(), 1);

    // A plan for some repositories has its own branch, so it leaves the shared
    // pull request alone instead of narrowing it.
    assert_ne!(branch_for(&first), GATHER_BRANCH);
    let partial = publish(&hub, "github:example/catalog", &first).unwrap();
    assert!(
        matches!(partial, Publication::PullRequest(_)),
        "{partial:?}"
    );
    publish(&hub, "github:example/catalog", &first).unwrap();

    let state = hub.repository("example/catalog").state();
    assert_eq!(state.prs.len(), 2);
    assert!(state.prs.iter().all(|pr| pr["state"] == "open"));
    let pr = &state.prs[0];
    assert_eq!(
        pr["title"],
        "feat(corpus): import tests from example/a and example/b"
    );
    // Each pull request opens in the review state its caller named:
    // ready on the shared branch here, a draft by default.
    assert_eq!(pr["draft"], false);
    assert_eq!(state.prs[1]["draft"], true);
    let body = pr["body"].as_str().unwrap();
    assert!(body.contains("| Add | `github:example/a/one` |"), "{body}");
    assert!(body.contains("| Add | `github:example/b/two` |"), "{body}");
}

#[test]
fn distributes_one_pull_request_per_receiving_repository() {
    let hub = FakeGitHubs::new(&["example/a", "example/r1", "example/r2", "example/plain"]);
    share(&hub, "example/a", "one");
    hub.push("example/r1", &[(MANIFEST, RECEIVING)]);
    hub.push("example/r2", &[(MANIFEST, RECEIVING)]);
    hub.push("example/plain", &[(MANIFEST, "schema=1\n")]);
    let root = tempfile::tempdir().unwrap();
    import(&hub, root.path(), &enrolled(&["example/a"]));
    let run = |name: &str, publish: bool| {
        let catalog = Catalog::load(root.path()).unwrap();
        let files = load_catalog_files(root.path(), &catalog).unwrap();
        let offer = publish.then(Offer::default);
        distribute(&hub, &enrolled(&[name])[0], &catalog, &files, offer)
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
    import(&hub, root.path(), &enrolled(&["example/a"]));
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
