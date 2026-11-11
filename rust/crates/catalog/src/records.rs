//! TOML exchange records with comment-preserving updates.

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use toml_edit::{DocumentMut, Item, Table};

/// Serialize a record as readable TOML; canonical digests use a separate encoding.
pub fn encode<T: Serialize>(value: &T) -> Result<String> {
    Ok(toml::to_string_pretty(value)?)
}

/// Decode a UTF-8 TOML record into its schema type.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(toml::from_str(std::str::from_utf8(bytes)?)?)
}

/// Update a TOML document while retaining comments on surviving fields and tables.
/// Removed fields lose their attached comments; array entries follow stable identities.
pub fn preserve(previous: &str, next: &str) -> Result<String> {
    let mut previous = previous.parse::<DocumentMut>()?;
    let next = next.parse::<DocumentMut>()?;
    merge_table(previous.as_table_mut(), next.as_table());
    Ok(previous.to_string())
}

/// Reconcile named fields without replacing unchanged syntax or its decoration.
fn merge_table(previous: &mut Table, next: &Table) {
    // toml_edit renders tables by their document positions, including array entries.
    // Carry the requested order forward along with each entry's existing comments.
    previous.set_position(next.position());
    previous.set_implicit(next.is_implicit());
    previous.retain(|key, _| next.contains_key(key));
    for (key, value) in next.iter() {
        if let Some(old) = previous.get_mut(key) {
            merge_item(old, value);
        } else {
            previous.insert(key, value.clone());
        }
    }
}

/// Identify array-of-table records across insertions, removals, and reordering.
fn identity(table: &Table) -> Option<(&str, &str)> {
    ["id", "name", "path"].into_iter().find_map(|key| {
        table
            .get(key)
            .and_then(Item::as_str)
            .map(|value| (key, value))
    })
}

/// Merge nested records and carry scalar comments forward when values change.
fn merge_item(previous: &mut Item, next: &Item) {
    match (previous, next) {
        (Item::Table(old), Item::Table(new)) => merge_table(old, new),
        (Item::ArrayOfTables(old), Item::ArrayOfTables(new)) => {
            let entries = old.clone();
            old.clear();
            for (index, table) in new.iter().enumerate() {
                let existing = if let Some(id) = identity(table) {
                    entries.iter().find(|entry| identity(entry) == Some(id))
                } else {
                    entries.get(index).filter(|entry| identity(entry).is_none())
                };
                let mut merged = existing.cloned().unwrap_or_else(|| table.clone());
                merge_table(&mut merged, table);
                old.push(merged);
            }
        }
        (Item::Value(old), Item::Value(new)) => {
            // Parse through TOML's semantic model so quoting and whitespace do not
            // force rewrites of otherwise unchanged user-authored values.
            let old_value = format!("value = {old}").parse::<toml::Table>();
            let new_value = format!("value = {new}").parse::<toml::Table>();
            if !matches!((&old_value, &new_value), (Ok(a), Ok(b)) if a == b) {
                let decor = old.decor().clone();
                *old = new.clone();
                *old.decor_mut() = decor;
            }
        }
        (old, new) => *old = new.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_comments_and_identity_when_records_change() {
        let before = "# Registry\nschema = 1 # version\n\n[[tests]] # first\nid = 'one'\nstatus = 'old' # explanation\n\n[[tests]] # second\nid = 'two'\nstatus = 'ok'\n";
        let after = "schema = 2\n[[tests]]\nid = 'two'\nstatus = 'ok'\n[[tests]]\nid = 'one'\nstatus = 'new'\n";
        let updated = preserve(before, after).unwrap();
        for comment in [
            "# Registry",
            "# version",
            "# first",
            "# second",
            "# explanation",
        ] {
            assert!(updated.contains(comment), "lost {comment}: {updated}");
        }
        assert_eq!(
            decode::<toml::Value>(updated.as_bytes()).unwrap(),
            decode::<toml::Value>(after.as_bytes()).unwrap()
        );
        assert_eq!(preserve(&updated, after).unwrap(), updated);
    }

    #[test]
    fn comments_and_formatting_do_not_change_record_digests() {
        let plain: toml::Value = decode(b"schema=1\nname='example'\n").unwrap();
        let annotated: toml::Value =
            decode(b"# Purpose\nschema = 1\nname = \"example\" # note\n").unwrap();
        assert_eq!(
            crate::digest(&plain).unwrap(),
            crate::digest(&annotated).unwrap()
        );
    }
    #[test]
    fn keeps_required_empty_tables_when_the_last_entry_is_removed() {
        let before = "schema = 1\n[tests.one]\nname = 'one'\n";
        let after = "schema = 1\n[tests]\n";
        let updated = preserve(before, after).unwrap();
        assert_eq!(
            decode::<toml::Value>(updated.as_bytes()).unwrap(),
            decode::<toml::Value>(after.as_bytes()).unwrap()
        );
    }
}
