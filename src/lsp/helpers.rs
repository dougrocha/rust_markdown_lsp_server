use std::str::FromStr;

use lsp_types::{Position, Range, Uri};
use miette::{Context, Result};
use ropey::RopeSlice;

use crate::{
    document::{
        references::{ReferenceKind, TargetHeader},
        Document,
    },
    lsp::server::Server,
    path::combine_and_normalize,
    Reference, TextBufferConversions,
};

/// Retrieves the content from a linked document based on the provided link data.
pub fn get_content(
    lsp: &Server,
    document: &Document,
    target: &str,
    header: Option<TargetHeader>,
) -> Result<String> {
    let file_path = combine_and_normalize(&document.uri, &Uri::from_str(target).unwrap())?;

    let document = lsp.documents.get_document(&file_path).context(format!(
        "Document '{:?}' not found in workspace",
        file_path.as_str()
    ))?;
    let slice = document.content.slice(..);

    let Some(header_target) = header else {
        return Ok(slice.to_string());
    };

    let (extracted_content, _range) =
        extract_header_section(&header_target, &document.references, slice);

    match extracted_content {
        Some(content) => Ok(content.to_string()),
        None => Ok(slice.to_string()),
    }
}

// TODO: Double check to see if the extraction is correct. Sometimes a header may extract a divider
// along with it. That is not intended
/// Extracts the content header section from the provided links.
pub fn extract_header_section<'a>(
    header: &TargetHeader,
    links: &[Reference],
    content: RopeSlice<'a>,
) -> (Option<RopeSlice<'a>>, Range) {
    let mut start_position: Option<Position> = None;
    let mut end_position: Option<Position> = None;

    for link in links {
        if let ReferenceKind::Header { level, content } = &link.kind {
            if start_position.is_none() && *content == header.content && *level == header.level {
                start_position = Some(link.range.start);
                continue;
            } else if start_position.is_some() && *level <= header.level {
                end_position = Some(link.range.start);
                break;
            }
        }
    }

    match (start_position, end_position) {
        (Some(start), Some(end)) if start < end && (end.line as usize) <= content.len_bytes() => {
            let start_idx = content.lsp_position_to_byte(start);
            let end_idx = content.lsp_position_to_byte(end);
            (
                Some(content.byte_slice(start_idx..end_idx)),
                Range::new(start, end),
            )
        }
        (Some(start), None) if (start.line as usize) < content.len_bytes() => {
            let start_idx = content.lsp_position_to_byte(start);
            (
                Some(content.byte_slice(start_idx..)),
                Range::new(start, Position::new(content.len_lines() as u32, 0)),
            )
        }
        _ => (None, Range::default()),
    }
}
