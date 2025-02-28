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

    for change in params.content_changes {
        match change {
            TextDocumentContentChangeEvent::Full(full_text_document_content_change) => {
                lsp.update_document(
                    &params.text_document.text_document_identifier.uri,
                    full_text_document_content_change.text,
                );
            }
            TextDocumentContentChangeEvent::Incremental(
                incremental_text_document_content_change,
            ) => todo!("Incremental Changes Not Supported!"),
        }
    }
}
