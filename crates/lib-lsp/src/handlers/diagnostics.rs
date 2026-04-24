use lsp_types::{
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport,
};
use miette::{Context, Result};

use crate::{get_document, server_state::ServerState};

pub fn process_diagnostic(
    lsp: &mut ServerState,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    let uri = params.text_document.uri;

    let document = get_document!(&lsp, &uri);

    Ok(DocumentDiagnosticReportResult::Report(
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: Some("markdown-lsp".to_owned()),
                items: document.diagnostics.clone(),
            },
        }),
    ))
}
