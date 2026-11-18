use crate::analyze;

#[test]
fn extracts_escaped_metadata_and_ignores_extension_evidence() {
    let text = r#";; License notice
(@cm-testsuite
  (; nested (; comment ;) ;)
  (schema 1)
  (intent "Check \"borrow\" lifetimes: caf\c3\a9")
  (requires "component-model" "experimental:resources" "component-model")
  (covers)
  (extensions (@vendor.example component resource shared v128 assert_invalid)))
(@unrelated (component) "ignored")
(module (@custom "notes" "opaque") (func))"#;
    let analysis = analyze(text).unwrap();
    let metadata = analysis.metadata.unwrap();
    assert_eq!(metadata.schema, 1);
    assert_eq!(metadata.intent, "Check \"borrow\" lifetimes: café");
    assert_eq!(
        metadata.requires.unwrap(),
        ["component-model".into(), "experimental:resources".into()].into()
    );
    assert!(metadata.covers.unwrap().is_empty());
    assert!(analysis.syntax.is_empty());
    assert_eq!(analysis.warnings.len(), 1);
    assert!(!analysis.complete);
}

#[test]
fn omitted_labels_and_quoted_annotation_names_are_supported() {
    let analysis = analyze(
        r#"(@"cm-testsuite" (schema 1) (intent "An empty component is valid.")) (component)"#,
    )
    .unwrap();
    let metadata = analysis.metadata.unwrap();
    assert!(metadata.requires.is_none());
    assert!(metadata.covers.is_none());
    assert_eq!(analysis.syntax.len(), 1);
    assert_eq!(analysis.syntax[0].feature, "component-model");
}

#[test]
fn rejects_invalid_metadata_contracts() {
    for (fields, message) in [
        ("(intent \"Purpose\")", "missing metadata schema"),
        ("(schema 1)", "missing metadata intent"),
        ("(schema 2) (intent \"Purpose\")", "unsupported"),
        ("(schema 1 2) (intent \"Purpose\")", "one integer"),
        ("(schema 1) (intent \" \\n\\t\")", "blank"),
        ("(schema 1) (intent \"\\ff\")", "UTF-8"),
        ("(schema 1) (intent 3)", "expected metadata string"),
        ("(schema 1) (intent \"One\" \"Two\")", "one string"),
        (
            "(schema 1) (intent \"Purpose\") (requires component-model)",
            "expected metadata string",
        ),
        ("(schema 1) (intent \"Purpose\") (covers \"\")", "blank"),
        ("(schema 1) (intent \"Purpose\") (schema 1)", "duplicate"),
        (
            "(schema 1) (intent \"Purpose\") (requires) (requires)",
            "duplicate",
        ),
        (
            "(schema 1) (intent \"Purpose\") (covers) (covers)",
            "duplicate",
        ),
        (
            "(schema 1) (intent \"Purpose\") (intent \"Again\")",
            "duplicate",
        ),
        (
            "(schema 1) (intent \"Purpose\") (features \"simd\")",
            "unknown metadata field",
        ),
        (
            "(schema 1) (intent \"Purpose\") (extensions (vendor \"x\"))",
            "must contain annotations",
        ),
        ("(schema 1) (intent \"Purpose\") (@typo)", "field name"),
    ] {
        let text = format!("(@cm-testsuite {fields}) (module)");
        let error = format!("{:#}", analyze(&text).unwrap_err());
        assert!(error.contains(message), "{text}: {error}");
    }
}

#[test]
fn rejects_duplicate_late_and_nested_headers() {
    let header = r#"(@cm-testsuite (schema 1) (intent "Purpose"))"#;
    for text in [
        format!("{header} {header} (module)"),
        format!("(module) {header}"),
        format!("(component {header})"),
        format!("(assert_invalid (module {header}) \"bad\")"),
    ] {
        assert!(analyze(&text).is_err(), "{text}");
    }
    assert!(analyze("(@cm-testsuite (schema 1) (module)").is_err());
    assert!(analyze(header).is_err());
}

#[test]
fn comments_strings_and_other_annotations_are_opaque() {
    let text = r#";; (@cm-testsuite (schema 999))
(@vendor (@cm-testsuite (schema 999)) component)
(assert_malformed (module quote "(@cm-testsuite broken") "bad")"#;
    let analysis = analyze(text).unwrap();
    assert!(analysis.metadata.is_none());
    assert!(analysis.syntax.is_empty());
    assert!(
        analysis
            .warnings
            .iter()
            .any(|w| w.contains("@cm-testsuite"))
    );
}

#[test]
fn annotation_tokens_do_not_change_evidence_offsets() {
    let text = "(@vendor component resource v128 assert_invalid)\n;; Intent comment remains visible\n(component)";
    let analysis = analyze(text).unwrap();
    assert_eq!(analysis.syntax.len(), 1);
    assert_eq!(
        analysis.syntax[0].byte_offset,
        text.rfind("component").unwrap()
    );
    assert!(
        !analysis
            .warnings
            .iter()
            .any(|w| w.contains("top-of-file") || w.contains("subtle assertions"))
    );
}
