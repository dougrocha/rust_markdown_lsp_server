use lsp_types::{TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier};
use miette::Result;
use serde::Deserialize;

use crate::message::Notification;

use super::server::Server;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams {
    text_document: VersionedTextDocumentIdentifier,
    content_changes: Vec<TextDocumentContentChangeEvent>,
}

pub fn process_did_change(lsp: &mut Server, notification: Notification) -> Result<()> {
    let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params).unwrap();

    let uri = params.text_document.uri;

    for change in params.content_changes {
        let TextDocumentContentChangeEvent {
            text,
            range,
            range_length,
        } = change;
        if range.is_none() && range_length.is_none() {
            lsp.documents.update_document(&uri, &text)?;
        } else {
            todo!("Incremental Changes Not Supported!");
        }
    }

    Ok(())
}
