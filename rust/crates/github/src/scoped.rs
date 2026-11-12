//! Route writes to each repository through a credential obtained for it.

use crate::Api;
use anyhow::{Context, Result};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap};

/// Open an API connection authorized to write to `github:owner/repository`.
pub type Connect<'a> = Box<dyn Fn(&str) -> Result<Box<dyn Api + 'a>> + 'a>;

/// An [`Api`] that reads with one credential and writes with per-repository ones.
///
/// A repository's write connection is opened on its first mutating request,
/// so a run asks for write access only where it actually publishes. Later
/// reads of that repository reuse the connection.
pub struct Scoped<'a> {
    reads: &'a dyn Api,
    connect: Connect<'a>,
    writers: RefCell<BTreeMap<String, Box<dyn Api + 'a>>>,
}

impl<'a> Scoped<'a> {
    pub fn new(reads: &'a dyn Api, connect: Connect<'a>) -> Self {
        Self {
            reads,
            connect,
            writers: RefCell::default(),
        }
    }

    /// Close the write connection to `github:owner/repository`, if one was opened.
    ///
    /// Call this once a repository's work is finished, so a connection that
    /// revokes its credential on drop does so then rather than at the end of the run.
    pub fn release(&self, repository: &str) {
        self.writers.borrow_mut().remove(repository);
    }
}

/// Name the `github:owner/repository` an API path addresses.
fn repository(path: &str) -> Result<String> {
    let mut parts = path
        .strip_prefix("/repos/")
        .context("invalid GitHub API path")?
        .split(['/', '?']);
    let owner = parts.next().filter(|part| !part.is_empty());
    let name = parts.next().filter(|part| !part.is_empty());
    Ok(format!(
        "github:{}/{}",
        owner.context("API path names no owner")?,
        name.context("API path names no repository")?
    ))
}

impl Api for Scoped<'_> {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let repository = repository(path)?;
        if method == "GET" && !self.writers.borrow().contains_key(&repository) {
            return self.reads.request(method, path, body);
        }
        if !self.writers.borrow().contains_key(&repository) {
            let writer = (self.connect)(&repository)?;
            self.writers.borrow_mut().insert(repository.clone(), writer);
        }
        self.writers.borrow()[&repository].request(method, path, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Record which connection served each request, and when it was closed.
    struct Named<'a>(&'a str, &'a RefCell<Vec<String>>);

    impl Drop for Named<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(format!("{} closed", self.0));
        }
    }

    impl Api for Named<'_> {
        fn request(&self, method: &str, path: &str, _: Option<Value>) -> Result<Option<Value>> {
            self.1
                .borrow_mut()
                .push(format!("{} {method} {path}", self.0));
            Ok(None)
        }
    }

    #[test]
    fn writes_open_one_connection_per_repository_and_reads_follow() {
        let log = RefCell::new(vec![]);
        let reads = Named("reader", &log);
        let opened = Cell::new(0);
        let api = Scoped::new(
            &reads,
            Box::new(|repository| {
                opened.set(opened.get() + 1);
                anyhow::ensure!(repository != "github:example/untrusted", "no trust policy");
                Ok(Box::new(Named("writer", &log)))
            }),
        );

        api.request("GET", "/repos/example/a/pulls?state=open", None)
            .unwrap();
        api.request("POST", "/repos/example/a/pulls", None).unwrap();
        api.request("PATCH", "/repos/example/a/pulls/1", None)
            .unwrap();
        api.request("GET", "/repos/example/a", None).unwrap();
        api.request("GET", "/repos/example/b", None).unwrap();
        // Releasing a repository closes its writer at once; reads go back to the reader.
        api.release("github:example/b");
        api.release("github:example/a");
        api.request("GET", "/repos/example/a", None).unwrap();
        assert_eq!(opened.get(), 1);
        assert_eq!(
            *log.borrow(),
            [
                "reader GET /repos/example/a/pulls?state=open",
                "writer POST /repos/example/a/pulls",
                "writer PATCH /repos/example/a/pulls/1",
                "writer GET /repos/example/a",
                "reader GET /repos/example/b",
                "writer closed",
                "reader GET /repos/example/a",
            ]
        );

        // A repository that grants no write access fails alone, on its first write.
        api.request("GET", "/repos/example/untrusted", None)
            .unwrap();
        assert!(
            api.request("POST", "/repos/example/untrusted/pulls", None)
                .is_err()
        );
        assert!(api.request("GET", "/user", None).is_err());
    }
}
