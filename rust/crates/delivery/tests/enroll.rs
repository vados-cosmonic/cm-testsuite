//! Enrollment from an issue form, against in-memory GitHub repositories.

use cm_catalog::{PARTICIPANTS_PATH, Participants};
use cm_delivery::{Enrollment, publish_enrollment, review_enrollment};
use cm_test_support::github::FakeGitHubs;
use serde_json::json;

const CATALOG: &str = "github:example/catalog";
const ISSUE: u64 = 7;
const BRANCH: &str = "cm-testsuite/enroll/7";
const REGISTRY: &str = "# Enrolled repositories.\nschema = 1\n";
const POLICY: &str = ".github/chainguard/cm-testsuite.sts.yaml";
const SHARING: &str = "schema=1\n[[test]]\nname='smoke'\npath='file:///tests/smoke.wast'\n\
    summary='Check smoke'\nrequires=['component-model']\ncovers=['smoke']\n";
const RECEIVING: &str = "schema=1\n[receive]\nenabled=true\n[[receive.route]]\nname='smoke'\n\
    destination='file:///shared'\nsupports=['component-model']\nallow_unknown=true\n";

/// A submitted form as GitHub renders it, with both roles ticked.
fn form(repository: &str, maintainers: &str, manifest: &str, reference: &str) -> String {
    format!(
        "### Repository\n\n{repository}\n\n### Roles\n\n\
         - [x] Submit tests (the manifest declares `[[test]]` entries)\n\
         - [x] Receive tests (the manifest sets `[receive] enabled = true`)\n\n\
         ### Maintainers\n\n{maintainers}\n\n### Manifest path\n\n{manifest}\n\n\
         ### Branch to read tests from\n\n{reference}\n\n### Setup\n\n\
         - [x] The manifest is committed on the branch named above.\n\
         - [x] I maintain this repository, or its maintainers have agreed to this request.\n\n\
         ### Anything else\n\nThanks!\n"
    )
}

/// A catalog with an empty registry and a project that shares one test,
/// with an issue asking to enroll the project.
fn hub() -> FakeGitHubs {
    let hub = FakeGitHubs::new(&["example/catalog", "example/project"]);
    hub.push("example/catalog", &[(PARTICIPANTS_PATH, REGISTRY)]);
    hub.push("example/project", &[(".cm-testsuite.toml", SHARING)]);
    file(
        &hub,
        &form("example/project", "octo-cat", "_No response_", "main"),
    );
    hub
}

/// File the enrollment issue, or rewrite it as its author could.
fn file(hub: &FakeGitHubs, body: &str) {
    let catalog = hub.repository("example/catalog");
    catalog
        .state_mut()
        .issues
        .retain(|issue| issue["number"] != ISSUE);
    catalog.add_issue(ISSUE, "[Enroll] example/project", body);
}

fn review(hub: &FakeGitHubs) -> anyhow::Result<Enrollment> {
    review_enrollment(hub, CATALOG, ISSUE)
}

/// The registry as proposed on the enrollment branch.
fn proposed(hub: &FakeGitHubs) -> String {
    let state = hub.repository("example/catalog").state();
    state.files_at(&state.refs[BRANCH])[PARTICIPANTS_PATH].clone()
}

fn warnings(enrollment: &Enrollment) -> Vec<&str> {
    enrollment
        .checks
        .iter()
        .filter(|check| !check.passed)
        .map(|check| check.summary.as_str())
        .collect()
}

#[test]
fn one_draft_pull_request_per_issue_is_updated_in_place() {
    let hub = hub();
    let catalog = hub.repository("example/catalog");
    let main = catalog.state().refs["main"].clone();

    // Reviewing reads only.
    let enrollment = review(&hub).unwrap();
    assert_eq!(hub.writes(), 0);
    assert_eq!(enrollment.participant.repository, "github:example/project");
    assert_eq!(enrollment.participant.reference.as_deref(), Some("main"));
    assert_eq!(enrollment.title(), "feat(registry): enroll example/project");

    let url = publish_enrollment(&hub, CATALOG, &enrollment).unwrap();
    {
        let state = catalog.state();
        assert_eq!(state.refs["main"], main);
        let [pr] = state.prs.as_slice() else {
            panic!("expected one pull request")
        };
        assert_eq!(pr["html_url"], url);
        assert_eq!(pr["draft"], true);
        assert_eq!(pr["head"], BRANCH);
        assert_eq!(pr["base"], "main");
        assert_eq!(pr["title"], "feat(registry): enroll example/project");
        let body = pr["body"].as_str().unwrap();
        assert!(
            body.contains("- [x] `.cm-testsuite.toml` at `main` ("),
            "{body}"
        );
        assert!(body.contains("- [x] The manifest shares 1 test."), "{body}");
        assert!(body.contains("- [ ] **Warning:** The issue asks to receive"));
        assert!(body.ends_with("\nCloses #7\n"), "{body}");
        // Maintainers are named in the diff, never mentioned in the description.
        assert!(!body.contains('@'), "{body}");

        // The branch changes the registry alone, on top of the default branch.
        let files = state.files_at(&state.refs[BRANCH]);
        let mut expected = state.files_at(&main).clone();
        expected.insert(
            PARTICIPANTS_PATH.into(),
            format!(
                "{REGISTRY}\n[[participant]]\nrepository = \"github:example/project\"\n\
                 reference = \"main\"\nmaintainers = [\"octo-cat\"]\n"
            ),
        );
        assert_eq!(files, &expected);
        let commit = &state.commits[&state.refs[BRANCH]];
        assert_eq!(
            commit.message,
            "feat(registry): enroll example/project\n\nCloses #7\n"
        );
        assert_eq!(commit.parents, std::slice::from_ref(&main));
    }

    // Repeating the run writes no commit and opens nothing new.
    let head = catalog.state().refs[BRANCH].clone();
    assert_eq!(
        publish_enrollment(&hub, CATALOG, &review(&hub).unwrap()).unwrap(),
        url
    );
    assert_eq!(catalog.state().refs[BRANCH], head);
    assert_eq!(catalog.state().prs.len(), 1);

    // An edited issue and a moved default branch update the same pull request.
    file(
        &hub,
        &form("example/project", "octo-cat, hubot", "", "_No response_"),
    );
    hub.push("example/catalog", &[("README", "moved")]);
    let moved = catalog.state().refs["main"].clone();
    assert_eq!(
        publish_enrollment(&hub, CATALOG, &review(&hub).unwrap()).unwrap(),
        url
    );
    let state = catalog.state();
    assert_eq!(state.prs.len(), 1);
    assert_eq!(state.commits[&state.refs[BRANCH]].parents, [head, moved]);
    assert_eq!(state.files_at(&state.refs[BRANCH])["README"], "moved");
    assert_eq!(
        proposed(&hub),
        format!(
            "{REGISTRY}\n[[participant]]\nrepository = \"github:example/project\"\n\
             maintainers = [\"octo-cat\", \"hubot\"]\n"
        )
    );
}

#[test]
fn requests_that_cannot_work_are_refused_before_any_write() {
    let hub = hub();
    let catalog = hub.repository("example/catalog");
    let refused = |reason: &str| {
        let error = format!("{:#}", review(&hub).expect_err(reason));
        assert_eq!(hub.writes(), 0, "{reason}");
        error
    };

    file(&hub, "Please enroll example/project.");
    assert!(refused("free text").contains("not a valid enrollment request"));
    file(&hub, &form("example/project; rm -rf /", "octo-cat", "", ""));
    assert!(refused("unsafe repository").contains("OWNER/REPO"));

    // The named repository, branch, and manifest must exist and parse.
    file(&hub, &form("example/missing", "octo-cat", "", ""));
    refused("missing repository");
    file(&hub, &form("example/project", "octo-cat", "", "absent"));
    assert!(refused("missing branch").contains("resolving absent"));
    file(
        &hub,
        &form("example/project", "octo-cat", "ci/exchange.toml", ""),
    );
    assert!(refused("missing manifest").contains("has no `ci/exchange.toml`"));
    hub.push("example/project", &[("ci/exchange.toml", "schema = 2\n")]);
    assert!(refused("invalid manifest").contains("invalid manifest"));

    // Only an open issue is acted on, and a pull request is not an issue.
    file(&hub, &form("example/project", "octo-cat", "", ""));
    review(&hub).unwrap();
    catalog.state_mut().issues[0]["state"] = json!("closed");
    assert!(refused("closed issue").contains("is not open"));
    catalog.state_mut().unavailable.insert(ISSUE, 404);
    assert!(refused("missing issue").contains("not found"));
    catalog.state_mut().unavailable.clear();
    catalog.state_mut().issues[0]["pull_request"] = json!({});
    assert!(refused("pull request").contains("pull request"));
}

#[test]
fn the_manifest_decides_what_is_reported_as_a_warning() {
    let hub = hub();
    // A receiving repository without its trust policy can still be enrolled.
    hub.push("example/project", &[(".cm-testsuite.toml", RECEIVING)]);
    let enrollment = review(&hub).unwrap();
    let found = warnings(&enrollment);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("asks to submit tests"), "{found:?}");
    assert!(found[1].contains("is missing on `main`"), "{found:?}");
    assert!(enrollment.checklist().contains("- [ ] **Warning:** "));

    hub.push("example/project", &[(POLICY, "issuer: example\n")]);
    let found = review(&hub).unwrap();
    assert_eq!(warnings(&found).len(), 1, "{found:?}");
    assert!(
        found
            .checks
            .iter()
            .any(|check| { check.passed && check.summary.contains("is committed on `main`") })
    );

    // The manifest is read at the requested branch; the policy on the default one.
    {
        let project = hub.repository("example/project");
        let mut state = project.state_mut();
        let main = state.refs["main"].clone();
        let mut files = state.files_at(&main).clone();
        files.insert(".cm-testsuite.toml".into(), "schema=1\n".into());
        let next = state.commit(files, "next", vec![main]);
        state.refs.insert("next".into(), next);
    }
    file(&hub, &form("example/project", "octo-cat", "", "next"));
    let found = review(&hub).unwrap();
    let found = warnings(&found);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[2].contains("neither shares nor receives"));
}

#[test]
fn an_enrolled_repository_is_not_proposed_again() {
    let hub = hub();
    let catalog = hub.repository("example/catalog");
    let enrollment = review(&hub).unwrap();
    publish_enrollment(&hub, CATALOG, &enrollment).unwrap();

    // Merge the proposal.
    {
        let mut state = catalog.state_mut();
        let merged = state.refs[BRANCH].clone();
        state.refs.insert("main".into(), merged);
        state.prs[0]["state"] = json!("closed");
    }
    let registry = proposed(&hub);
    assert_eq!(
        Participants::parse(registry.as_bytes())
            .unwrap()
            .participant,
        std::slice::from_ref(&enrollment.participant)
    );
    let writes = hub.writes();
    let error = publish_enrollment(&hub, CATALOG, &enrollment).unwrap_err();
    assert!(error.to_string().contains("already enrolled"), "{error}");
    assert_eq!(hub.writes(), writes);
}

#[test]
fn work_by_others_on_the_branch_or_pull_request_is_left_alone() {
    let hub = hub();
    let catalog = hub.repository("example/catalog");
    let enrollment = review(&hub).unwrap();
    publish_enrollment(&hub, CATALOG, &enrollment).unwrap();
    file(&hub, &form("example/project", "octo-cat, hubot", "", ""));
    let changed = review(&hub).unwrap();
    let refused = |reason: &str| {
        let writes = hub.writes();
        let head = catalog.state().refs[BRANCH].clone();
        let error = publish_enrollment(&hub, CATALOG, &changed).expect_err(reason);
        assert_eq!(hub.writes(), writes, "{reason}");
        assert_eq!(catalog.state().refs[BRANCH], head, "{reason}");
        error.to_string()
    };

    // A pull request on the branch that this command did not open.
    let body = catalog.state().prs[0]["body"].clone();
    catalog.state_mut().prs[0]["body"] = json!("Enroll by hand.");
    assert!(refused("foreign pull request").contains("not opened by this command"));
    catalog.state_mut().prs[0]["body"] = body;

    // A reviewer's commit on top of the proposal.
    {
        let mut state = catalog.state_mut();
        let old = state.refs[BRANCH].clone();
        let mut files = state.files_at(&old).clone();
        files.insert(PARTICIPANTS_PATH.into(), "schema = 1\n".into());
        let edited = state.commit(files, "fix(registry): adjust by hand", vec![old]);
        state.refs.insert(BRANCH.into(), edited);
    }
    assert!(refused("foreign commit").contains("did not write"));
}
