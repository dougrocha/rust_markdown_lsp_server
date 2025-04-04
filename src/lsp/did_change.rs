use serde::Deserialize;

use crate::{message::Notification, LspServer};

use super::{TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier};

#[derive(Deserialize, Debug)]
pub struct DidChangeTextDocumentParams {
    #[serde(rename = "textDocument")]
    text_document: VersionedTextDocumentIdentifier,
    #[serde(rename = "contentChanges")]
    content_changes: Vec<TextDocumentContentChangeEvent>,
}

pub fn process_did_change(lsp: &mut LspServer, notification: Notification) {
    let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params).unwrap();

    let uri = params.text_document.text_document_identifier.uri;

    for change in params.content_changes {
        match change {
            TextDocumentContentChangeEvent::Full(full_text_document_content_change) => {
                lsp.update_document(&uri, &full_text_document_content_change.text);
            }
            TextDocumentContentChangeEvent::Incremental(
                _incremental_text_document_content_change,
            ) => todo!("Incremental Changes Not Supported!"),
        }
    }
}
