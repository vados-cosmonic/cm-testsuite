//! Read a submitted "Enroll a repository" issue form as a registry entry.
//!
//! Anyone can open or edit an issue, so nothing here trusts the text. A value
//! is used only after it passes validation, and no error repeats it.

use anyhow::{Context, Result, bail, ensure};
use cm_catalog::{Participant, validate_login, validate_name, validate_source};

/// What issue titles from the form start with.
const TITLE: &str = "[Enroll]";

/// The form's field labels, in order, as GitHub renders them under `###` headings.
/// Keep them in step with `.github/ISSUE_TEMPLATE/enroll-repository.yml`.
const FIELDS: [&str; 7] = [
    "Repository",
    "Roles",
    "Maintainers",
    "Manifest path",
    "Branch to read tests from",
    "Setup",
    "Anything else",
];

/// What GitHub renders for an optional field left empty.
const EMPTY: &str = "_No response_";

/// The role checkboxes, by how their labels start.
const SUBMIT: &str = "Submit tests";
const RECEIVE: &str = "Receive tests";

/// The setup checkboxes the form requires.
const CONFIRMATIONS: [&str; 2] = [
    "The manifest is committed on the branch named above.",
    "I maintain this repository, or its maintainers have agreed to this request.",
];

/// The manifest path the form and the registry assume when none is given.
const DEFAULT_MANIFEST: &str = ".cm-testsuite.toml";

/// More maintainers than this is a mistake or an attempt to mention bystanders.
const MAX_MAINTAINERS: usize = 20;

/// A validated enrollment request.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Request {
    /// The registry entry the issue asks for.
    pub(super) participant: Participant,
    /// Whether the issue says the repository submits tests.
    pub(super) submits: bool,
    /// Whether the issue says the repository receives tests.
    pub(super) receives: bool,
}

/// Split the body into the non-blank lines of each field before the free-text one.
///
/// The headings must be exactly the form's, once each and in order. Everything
/// after the last heading is free text, where a heading means nothing.
fn sections(body: &str) -> Result<Vec<Vec<&str>>> {
    let mut sections: Vec<Vec<&str>> = vec![];
    let mut complete = false;
    for line in body.lines().map(str::trim) {
        if let Some(heading) = line.strip_prefix("### ") {
            ensure!(
                FIELDS.get(sections.len()) == Some(&heading.trim()),
                "the body does not have the enrollment form's fields in order"
            );
            if sections.len() + 1 == FIELDS.len() {
                complete = true;
                break;
            }
            sections.push(vec![]);
        } else if !line.is_empty() {
            sections
                .last_mut()
                .context("the body does not start with the enrollment form's first field")?
                .push(line);
        }
    }
    ensure!(complete, "the body is missing enrollment form fields");
    Ok(sections)
}

/// The value of a single-line field; `None` when it was left empty.
fn value<'a>(field: &str, lines: &[&'a str]) -> Result<Option<&'a str>> {
    match lines {
        [] => Ok(None),
        [line] if *line == EMPTY => Ok(None),
        [line] => Ok(Some(line)),
        _ => bail!("the {field} field must be a single line"),
    }
}

/// The labels of a checkbox field with whether each is ticked.
fn checkboxes<'a>(field: &str, lines: &[&'a str]) -> Result<Vec<(&'a str, bool)>> {
    lines
        .iter()
        .filter(|line| **line != EMPTY)
        .map(|line| {
            if let Some(label) = line.strip_prefix("- [ ] ") {
                Ok((label.trim(), false))
            } else if let Some(label) = line
                .strip_prefix("- [x] ")
                .or_else(|| line.strip_prefix("- [X] "))
            {
                Ok((label.trim(), true))
            } else {
                bail!("the {field} field must hold only the form's checkboxes")
            }
        })
        .collect()
}

/// Read an issue's title and body as an enrollment request.
pub(super) fn parse(title: &str, body: &str) -> Result<Request> {
    ensure!(
        title.trim_start().starts_with(TITLE),
        "the title does not start with {TITLE}"
    );
    let sections = sections(body)?;
    let [repository, roles, maintainers, manifest, reference, setup] = sections.as_slice() else {
        bail!("the body is missing enrollment form fields");
    };

    let repository = value(FIELDS[0], repository)?
        .map(|name| format!("github:{name}"))
        .filter(|source| validate_source(source).is_ok())
        .context("the Repository field must be a GitHub repository as OWNER/REPO")?;

    let roles = checkboxes(FIELDS[1], roles)?;
    let ticked = |start: &str| {
        roles
            .iter()
            .any(|(label, ticked)| *ticked && label.starts_with(start))
    };

    // A leading @ and stray commas are how people type lists of logins.
    let mut logins: Vec<String> = vec![];
    for login in value(FIELDS[2], maintainers)?
        .unwrap_or_default()
        .split(',')
        .map(|login| login.trim())
        .filter(|login| !login.is_empty())
    {
        let login = login.strip_prefix('@').unwrap_or(login);
        ensure!(
            validate_login(login).is_ok(),
            "the Maintainers field must hold GitHub logins separated by commas"
        );
        if !logins.iter().any(|known| known.eq_ignore_ascii_case(login)) {
            logins.push(login.to_owned());
        }
    }
    ensure!(
        !logins.is_empty() && logins.len() <= MAX_MAINTAINERS,
        "the Maintainers field must name between 1 and {MAX_MAINTAINERS} GitHub logins"
    );

    // Both values reach API routes and Markdown, so they are held to the
    // conservative character set of test names rather than to all Git allows.
    let manifest = value(FIELDS[3], manifest)?.unwrap_or(DEFAULT_MANIFEST);
    ensure!(
        validate_name(manifest).is_ok() && manifest.len() <= 255,
        "the Manifest path field must be a repository-relative path of \
         ASCII letters, digits, -, _, ., and /"
    );
    let reference = value(FIELDS[4], reference)?;
    if let Some(reference) = reference {
        ensure!(
            validate_name(reference).is_ok()
                && !reference.starts_with('-')
                && reference.len() <= 255,
            "the Branch field must be a branch name of ASCII letters, digits, -, _, ., and /"
        );
    }

    let setup = checkboxes(FIELDS[5], setup)?;
    for confirmation in CONFIRMATIONS {
        ensure!(
            setup.contains(&(confirmation, true)),
            "the Setup field does not confirm: {confirmation}"
        );
    }

    let participant = Participant {
        repository,
        manifest: manifest.to_owned(),
        reference: reference.map(str::to_owned),
        maintainers: logins,
    };
    participant.validate()?;
    Ok(Request {
        participant,
        submits: ticked(SUBMIT),
        receives: ticked(RECEIVE),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render a submitted form the way GitHub does.
    fn form(repository: &str, maintainers: &str, manifest: &str, reference: &str) -> String {
        format!(
            "### Repository\n\n{repository}\n\n### Roles\n\n\
             - [x] Submit tests (the manifest declares `[[test]]` entries)\n\
             - [ ] Receive tests (the manifest sets `[receive] enabled = true`)\n\n\
             ### Maintainers\n\n{maintainers}\n\n### Manifest path\n\n{manifest}\n\n\
             ### Branch to read tests from\n\n{reference}\n\n### Setup\n\n\
             - [x] The manifest is committed on the branch named above.\n\
             - [x] I maintain this repository, or its maintainers have agreed to this request.\n\
             - [ ] Submitted tests are original to the repository or may be redistributed \
             under its license.\n\n### Anything else\n\n_No response_\n"
        )
    }

    fn entry(body: &str) -> Result<Participant> {
        parse("[Enroll] example/project", body).map(|request| request.participant)
    }

    #[test]
    fn reads_fields_and_leaves_defaults_implicit() {
        let request = parse(
            "[Enroll] example/project",
            &form(
                "example/project",
                "@octo-cat, hubot,Octo-Cat,",
                EMPTY,
                EMPTY,
            ),
        )
        .unwrap();
        assert_eq!(
            request.participant,
            Participant {
                repository: "github:example/project".into(),
                manifest: DEFAULT_MANIFEST.into(),
                reference: None,
                maintainers: vec!["octo-cat".into(), "hubot".into()],
            }
        );
        assert!(request.submits && !request.receives);

        // GitHub separates lines with CRLF in bodies submitted from a browser.
        let custom = form(
            "example/project",
            "hubot",
            "ci/exchange.toml",
            "release/1.x",
        );
        let custom = entry(&custom.replace('\n', "\r\n")).unwrap();
        assert_eq!(custom.manifest, "ci/exchange.toml");
        assert_eq!(custom.reference.as_deref(), Some("release/1.x"));
    }

    #[test]
    fn free_text_cannot_supply_or_override_fields() {
        let body = form("example/project", "hubot", EMPTY, EMPTY);
        let noted = format!("{body}\n### Repository\n\nexample/other\n");
        assert_eq!(entry(&noted).unwrap().repository, "github:example/project");

        // A field smuggled in before the form, or repeated inside it, is refused.
        assert!(entry(&format!("### Repository\n\nexample/other\n\n{body}")).is_err());
        assert!(entry(&format!("example/other\n\n{body}")).is_err());
        let repeated = body.replace("### Roles", "### Repository\n\nexample/other\n\n### Roles");
        assert!(entry(&repeated).is_err());
    }

    #[test]
    fn rejects_what_the_form_would_not_produce() {
        let valid = form("example/project", "hubot", EMPTY, EMPTY);
        assert!(parse("Please enroll example/project", &valid).is_err());
        assert!(entry("Enroll example/project, thanks!").is_err());
        assert!(entry(&valid.replace("### Setup", "### Set up")).is_err());
        assert!(entry(&valid.replace("### Anything else", "")).is_err());
        // Required confirmations must still be ticked after any later edit.
        assert!(entry(&valid.replace("- [x] I maintain", "- [ ] I maintain")).is_err());
        assert!(entry(&valid.replace("- [x] I maintain", "I maintain")).is_err());

        for repository in [
            EMPTY,
            "project",
            "example/project/extra",
            "github:example/project",
            "https://github.com/example/project",
            "example/project\nexample/other",
            "example/pro ject",
            "example/..",
            "example/project\"\nmaintainers = [\"x\"]",
            "example/`project`",
            "example/<b>project</b>",
        ] {
            let body = form(repository, "hubot", EMPTY, EMPTY);
            assert!(entry(&body).is_err(), "{repository}");
        }
        for maintainers in [EMPTY, ",", "octo cat", "a/b", "-dash", "@@twice", "[x](y)"] {
            let body = form("example/project", maintainers, EMPTY, EMPTY);
            assert!(entry(&body).is_err(), "{maintainers}");
        }
        let crowd = vec!["hubot"; MAX_MAINTAINERS + 1]
            .iter()
            .enumerate()
            .map(|(index, login)| format!("{login}{index}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(entry(&form("example/project", &crowd, EMPTY, EMPTY)).is_err());

        for path in [
            "/etc/passwd",
            "../x.toml",
            "a\\b",
            ".git/config",
            "a b",
            "a`b",
            "a@b",
        ] {
            let manifest = form("example/project", "hubot", path, EMPTY);
            assert!(entry(&manifest).is_err(), "manifest {path}");
            let reference = form("example/project", "hubot", EMPTY, path);
            assert!(entry(&reference).is_err(), "reference {path}");
        }
        let option = form("example/project", "hubot", EMPTY, "--upload-pack");
        assert!(entry(&option).is_err());
    }

    #[test]
    fn errors_do_not_repeat_the_submitted_text() {
        let marker = "attacker-text";
        for body in [
            form(&format!("{marker}/a b"), "hubot", EMPTY, EMPTY),
            form("example/project", &format!("{marker} x"), EMPTY, EMPTY),
            form("example/project", "hubot", &format!("{marker} x"), EMPTY),
            form("example/project", "hubot", EMPTY, &format!("{marker} x")),
            format!("### {marker}\n"),
        ] {
            let error = format!("{:#}", entry(&body).unwrap_err());
            assert!(!error.contains(marker), "{error}");
        }
    }
}
