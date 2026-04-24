use lsp_types::RenameFilesParams;
use miette::Result;
use tracing::trace;

use crate::ServerState;

pub fn process_did_rename(_lsp: &mut ServerState, params: RenameFilesParams) -> Result<()> {
    trace!(?params);

    Ok(())
}
