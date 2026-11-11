//! Render submitted observations without inferring execution results.

use crate::Report;
use anyhow::{Result, ensure};
use cm_catalog::*;

/// Escape user-supplied text before embedding it in HTML.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Show all supplied runs explicitly; no 'latest' or conformance claims are inferred.
pub fn render(catalog: &Catalog, reports: &[Report]) -> Result<String> {
    catalog.validate()?;
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>cm-testsuite results</title><style>body{font:16px system-ui;max-width:1100px;margin:3rem auto;padding:0 1rem}table{border-collapse:collapse;width:100%}td,th{padding:.6rem;border:1px solid #ccc;text-align:left}code{overflow-wrap:anywhere}</style><h1>Engine-reported results</h1><p>Submitted observations; cm-testsuite does not execute these tests. Missing entries mean not reported.</p>",
    );
    if reports.is_empty() {
        html.push_str("<p>No engine reports have been submitted.</p>");
    }
    for report in reports {
        ensure!(
            report.catalog_sha256 == digest(catalog)?,
            "cannot render a report against a different catalog"
        );
        // Escape publisher-controlled metadata before placing it in the run header.
        html.push_str(&format!(
            "<h2>{} {}</h2><p>Publisher: {} · Run: {} · Reported: {} · {} · {}</p>\
             <table><tr><th>Test</th><th>SHA-256</th><th>Status</th><th>Diagnostic</th></tr>",
            escape(&report.engine),
            escape(&report.engine_version),
            escape(&report.publisher),
            escape(&report.run_id),
            escape(&report.reported_at),
            escape(&report.platform),
            escape(&report.configuration),
        ));

        // Include unreported catalog entries explicitly instead of inferring a result.
        for (id, test) in &catalog.tests {
            let observation = report.results.iter().find(|x| &x.id == id);
            let status = observation
                .map(|entry| entry.status.as_str())
                .unwrap_or("not reported");
            html.push_str(&format!(
                "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                escape(id),
                test.sha256,
                escape(status),
                escape(observation.map(|x| x.diagnostic.as_str()).unwrap_or(""))
            ));
        }
        html.push_str("</table>");
    }
    html.push_str("</html>\n");
    Ok(html)
}
