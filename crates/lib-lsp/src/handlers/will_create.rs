use crate::ServerState;
use lsp_types::{CreateFilesParams, WorkspaceEdit};
use miette::Result;

// TODO: this will handle potential actions like creating a file with yaml metadata already in it
pub fn process_will_create_files(
    lsp: &mut ServerState,
    params: CreateFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    Ok(None)
}

pub fn process_did_create(lsp: &mut ServerState, params: CreateFilesParams) -> Result<()> {
    Ok(())
}
