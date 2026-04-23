use lsp_types::RenameFilesParams;
use miette::Result;
use tracing::trace;

use crate::Server;

pub fn process_did_rename(_lsp: &mut Server, params: RenameFilesParams) -> Result<()> {
    trace!(?params);

    Ok(())
}
