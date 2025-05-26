use lsp_types::{
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport,
};
use miette::{Context, Result};

use super::state::LspState;

pub fn process_diagnostic(
    lsp: &mut LspState,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    let uri = params.text_document.uri;
    let document = lsp.documents.get_document(&uri).context(format!(
        "Document '{:?}' not found in workspace",
        uri.as_str()
    ))?;

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
