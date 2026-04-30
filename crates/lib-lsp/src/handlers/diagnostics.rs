use gen_lsp_types::{
    DocumentDiagnosticParams, DocumentDiagnosticReport, FullDocumentDiagnosticReport,
    RelatedFullDocumentDiagnosticReport,
};
use miette::{Context, Result};

use crate::{get_document, server_state::ServerState, uri::UriExt};

pub fn process_diagnostic(
    lsp: &mut ServerState,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReport> {
    let uri = params.text_document.uri;

    let document = get_document!(&lsp, &uri);

    Ok(
        DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: Some("markdown-lsp".to_owned()),
                    items: document.diagnostics.clone(),
                },
            },
        ),
    )
}
