use gen_lsp_types::{DidChangeTextDocumentParams, TextDocumentContentChangeEvent};
use miette::{Result, miette};

use crate::{server_state::ServerState, uri::UriExt};

pub fn process_did_change(
    lsp: &mut ServerState,
    params: DidChangeTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.text_document_identifier.uri;
    let version = params.text_document.version;

    let path = uri
        .to_file_path()
        .ok_or_else(|| miette!("Invalid URI: {}", uri.as_ref()))?;

    for change in params.content_changes {
        match change {
            TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(event) => {
                lsp.documents.update_document(&path, version, &event.text)?;
            }
            TextDocumentContentChangeEvent::TextDocumentContentChangePartial(_) => {
                return Err(miette!(
                    "Incremental document changes are not yet supported.\nPlease configure your editor to send full document updates."
                ));
            }
        }
    }

    Ok(())
}
