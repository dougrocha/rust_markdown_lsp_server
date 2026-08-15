use gen_lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentDiagnosticParams, DocumentDiagnosticReport,
    FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport,
};
use lib_core::{
    config::FrontmatterFieldType,
    document::{
        Document,
        index::{Link, LinkKind, Severity},
    },
    path::slug::header_slug,
};
use miette::{Context, Result};

use crate::{
    get_document, handlers::link_resolver::resolve_target_uri, server_state::ServerState,
    text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

pub fn process_diagnostic(
    lsp: &mut ServerState,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReport> {
    let uri = params.text_document.uri;

    let document = get_document!(&lsp, &uri);

    let items = match uri.to_file_path() {
        Some(path) if lsp.is_document_open(&path) => {
            let mut items = computed_diagnostics(document);
            items.extend(broken_link_diagnostics(lsp, document));
            items.extend(missing_frontmatter_diagnostics(lsp, document));
            items.extend(frontmatter_schema_diagnostics(lsp, document));
            items
        }
        _ => vec![],
    };

    Ok(
        DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: Some("markdown-lsp".to_owned()),
                    items,
                },
            },
        ),
    )
}

fn computed_diagnostics(document: &Document) -> Vec<Diagnostic> {
    let slice = document.source.slice(..);

    document
        .diagnostics()
        .map(|diag| Diagnostic {
            range: slice.byte_to_lsp_range(diag.span),
            severity: Some(to_lsp_severity(diag.severity)),
            code: None,
            code_description: None,
            source: Some("markdown-lsp".to_owned()),
            message: diag.message.clone(),
            tags: None,
            related_information: None,
            data: None,
        })
        .collect()
}

/// True for links that point outside the workspace (URLs, mailto:, etc.) and
/// therefore can't be validated without a network request.
fn is_external_link(target: &str) -> bool {
    target.contains("://") || target.starts_with("mailto:")
}

fn missing_frontmatter_diagnostics(lsp: &ServerState, document: &Document) -> Vec<Diagnostic> {
    if !lsp.config.frontmatter.enabled || document.has_frontmatter() {
        return vec![];
    }

    let slice = document.source.slice(..);

    vec![Diagnostic {
        range: slice.byte_to_lsp_range(0..0),
        severity: Some(DiagnosticSeverity::Warning),
        code: None,
        code_description: None,
        source: Some("markdown-lsp".to_owned()),
        message: "Missing or invalid YAML frontmatter".to_owned(),
        tags: None,
        related_information: None,
        data: None,
    }]
}

fn frontmatter_schema_diagnostics(lsp: &ServerState, document: &Document) -> Vec<Diagnostic> {
    if !lsp.config.frontmatter.enabled || !document.has_frontmatter() {
        return vec![];
    }

    let slice = document.source.slice(..);

    lsp.config
        .frontmatter
        .schema
        .iter()
        .filter_map(|field| {
            let message = match document.frontmatter.get(&field.key) {
                None => format!("Missing required frontmatter field '{}'", field.key),
                Some(value) => {
                    let matches = match field.r#type {
                        FrontmatterFieldType::String => value.as_string().is_some(),
                        FrontmatterFieldType::List => value.as_list().is_some(),
                    };
                    if matches {
                        return None;
                    }
                    format!(
                        "Frontmatter field '{}' should be a {:?}",
                        field.key, field.r#type
                    )
                }
            };

            Some(Diagnostic {
                range: slice.byte_to_lsp_range(0..0),
                severity: Some(DiagnosticSeverity::Warning),
                code: None,
                code_description: None,
                source: Some("markdown-lsp".to_owned()),
                message,
                tags: None,
                related_information: None,
                data: None,
            })
        })
        .collect()
}

fn broken_link_diagnostics(lsp: &ServerState, document: &Document) -> Vec<Diagnostic> {
    if !lsp.config.diagnostics.enable_broken_links {
        return vec![];
    }

    let slice = document.source.slice(..);

    document
        .links()
        .filter_map(|link| check_link(lsp, document, link))
        .map(|(link, message)| Diagnostic {
            range: slice.byte_to_lsp_range(link.span),
            severity: Some(DiagnosticSeverity::Warning),
            code: None,
            code_description: None,
            source: Some("markdown-lsp".to_owned()),
            message,
            tags: None,
            related_information: None,
            data: None,
        })
        .collect()
}

/// Returns `Some((link, message))` if `link` is broken.
fn check_link<'a>(
    lsp: &ServerState,
    document: &Document,
    link: &'a Link,
) -> Option<(&'a Link, String)> {
    // Images aren't tracked as documents in the vault, so they're out of
    // scope for this check.
    if matches!(link.kind, LinkKind::Image { .. }) {
        return None;
    }

    let target = link.target_str(&document.source);
    if is_external_link(&target) {
        return None;
    }

    let target_uri = match resolve_target_uri(lsp, document, &target) {
        Ok(uri) => uri,
        Err(_) => {
            return Some((link, format!("Broken link: '{target}' could not be resolved")));
        }
    };

    let Some(target_path) = target_uri.to_file_path() else {
        return Some((link, format!("Broken link: '{target}' could not be resolved")));
    };

    let target_doc = lsp.documents.get_document(&target_path);
    if target_doc.is_none() && !target_path.exists() {
        return Some((link, format!("Broken link: '{target}' does not exist")));
    }

    let header = link.header_str(&document.source)?;
    let target_doc = target_doc?;

    let normalized_target = header_slug(&header);
    let header_exists = target_doc.headers().any(|h| {
        let content = h.content_str(&target_doc.source);
        content == header || header_slug(&content) == normalized_target
    });

    if header_exists {
        None
    } else {
        Some((
            link,
            format!("Broken link: header '#{header}' not found in '{target}'"),
        ))
    }
}

fn to_lsp_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::Error,
        Severity::Warning => DiagnosticSeverity::Warning,
        Severity::Info => DiagnosticSeverity::Information,
        Severity::Hint => DiagnosticSeverity::Hint,
    }
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{PartialResultParams, TextDocumentIdentifier, WorkDoneProgressParams};
    use lib_core::config::FrontmatterFieldSchema;

    use super::*;
    use crate::test_utils::TestWorkspace;

    fn params(uri: &str) -> DocumentDiagnosticParams {
        DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier {
                uri: uri.parse().unwrap(),
            },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    #[test]
    fn flags_malformed_tag_with_dangling_separator() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "an #area/ tag and a #clean tag");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].severity, Some(DiagnosticSeverity::Warning));
        assert!(items[0].message.contains("#area/"));
    }

    #[test]
    fn no_diagnostics_for_well_formed_tags() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "#clean and #area/sub tags");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn flags_broken_wikilink_to_missing_document() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "See [[missing]] for details");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("missing"));
    }

    #[test]
    fn no_diagnostic_for_wikilink_to_existing_document() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "See [[target]] for details")
            .add_file("/workspace/target.md", 1, "# Target\n\nBody");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn flags_broken_header_fragment() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "See [[target#Missing]] for details")
            .add_file("/workspace/target.md", 1, "# Target\n\n## Section");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("Missing"));
    }

    #[test]
    fn no_diagnostic_for_matching_header_fragment() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "See [[target#Section]] for details")
            .add_file("/workspace/target.md", 1, "# Target\n\n## Section");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_diagnostic_for_external_link() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "[docs](https://example.com/x)");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_diagnostic_for_broken_image() {
        let mut ws = TestWorkspace::new();
        ws.state.config.frontmatter.enabled = false;
        ws.open_file("/workspace/notes.md", 1, "![alt](missing.png)");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_diagnostics_for_documents_not_open() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "an #area/ tag");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn flags_missing_frontmatter_when_enabled() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/notes.md", 1, "# No frontmatter here");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("frontmatter"));
    }

    #[test]
    fn no_diagnostic_for_valid_frontmatter_when_enabled() {
        let mut ws = TestWorkspace::new();
        ws.open_file(
            "/workspace/notes.md",
            1,
            "---\ntitle: Hello\n---\n# Heading",
        );

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_missing_frontmatter_diagnostic_when_disabled() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/notes.md", 1, "# No frontmatter here");
        ws.state.config.frontmatter.enabled = false;

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn flags_missing_required_schema_field() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/notes.md", 1, "---\ntitle: Hello\n---\n# Heading");
        ws.state.config.frontmatter.schema = vec![FrontmatterFieldSchema {
            key: "tags".to_owned(),
            r#type: FrontmatterFieldType::List,
        }];

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("tags"));
    }

    #[test]
    fn flags_wrong_type_schema_field() {
        let mut ws = TestWorkspace::new();
        ws.open_file(
            "/workspace/notes.md",
            1,
            "---\ntitle: Hello\ntags: not-a-list\n---\n# Heading",
        );
        ws.state.config.frontmatter.schema = vec![FrontmatterFieldSchema {
            key: "tags".to_owned(),
            r#type: FrontmatterFieldType::List,
        }];

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("tags"));
    }

    #[test]
    fn no_diagnostic_when_schema_satisfied() {
        let mut ws = TestWorkspace::new();
        ws.open_file(
            "/workspace/notes.md",
            1,
            "---\ntitle: Hello\ntags:\n  - a\n  - b\n---\n# Heading",
        );
        ws.state.config.frontmatter.schema = vec![
            FrontmatterFieldSchema {
                key: "title".to_owned(),
                r#type: FrontmatterFieldType::String,
            },
            FrontmatterFieldSchema {
                key: "tags".to_owned(),
                r#type: FrontmatterFieldType::List,
            },
        ];

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_schema_violations_when_schema_empty() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/notes.md", 1, "---\ntitle: Hello\n---\n# Heading");

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[test]
    fn no_schema_violations_when_frontmatter_missing() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/notes.md", 1, "# No frontmatter here");
        ws.state.config.frontmatter.schema = vec![FrontmatterFieldSchema {
            key: "title".to_owned(),
            r#type: FrontmatterFieldType::String,
        }];

        let report =
            process_diagnostic(&mut ws.state, params("file:///workspace/notes.md")).unwrap();

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(report) = report else {
            panic!("expected a full report");
        };

        // only the missing-frontmatter diagnostic fires, not a schema violation
        let items = report.full_document_diagnostic_report.items;
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("frontmatter"));
    }
}
