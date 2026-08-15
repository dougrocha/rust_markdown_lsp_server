use gen_lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentDiagnosticParams, DocumentDiagnosticReport,
    FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport,
};
use lib_core::document::{Document, index::Severity};
use miette::{Context, Result};

use crate::{
    get_document, server_state::ServerState, text_buffer_conversions::TextBufferConversions,
    uri::UriExt,
};

pub fn process_diagnostic(
    lsp: &mut ServerState,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReport> {
    let uri = params.text_document.uri;

    let document = get_document!(&lsp, &uri);

    let items = match uri.to_file_path() {
        Some(path) if lsp.is_document_open(&path) => computed_diagnostics(document),
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
        ws.open_file("/workspace/notes.md", 1, "#clean and #area/sub tags");

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
}
