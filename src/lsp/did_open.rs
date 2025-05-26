use lsp_types::TextDocumentItem;
use miette::Result;
use serde::Deserialize;

use crate::message::Notification;

use super::server::Server;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    text_document: TextDocumentItem,
}

pub fn process_did_open(lsp: &mut Server, notification: Notification) -> Result<()> {
    let did_open_params: DidOpenTextDocumentParams =
        serde_json::from_value(notification.params).unwrap();

    lsp.documents.open_document(
        did_open_params.text_document.uri,
        did_open_params.text_document.version,
        &did_open_params.text_document.text,
    )?;

    Ok(())
}
