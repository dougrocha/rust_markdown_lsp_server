use miette::{Context, Result};
use ropey::RopeSlice;

use crate::{
    document::references::{combine_uri_and_relative_path, LinkData, LinkHeader},
    lsp::server::LspServer,
    Reference,
};

/// Retrieves the content from a linked document based on the provided link data.
pub fn get_content(lsp: &LspServer, link_data: &LinkData) -> Result<String> {
    let filepath = combine_uri_and_relative_path(link_data)
        .context("Failed to combine URI and relative path")?;
    let document = lsp
        .get_document(filepath.to_string_lossy())
        .ok_or_else(|| miette::miette!("Document not found"))?;
    let slice = document.content.slice(..);

    if link_data.header.is_none() {
        return Ok(slice.to_string());
    }

    let linked_doc = lsp
        .get_document(filepath.to_string_lossy())
        .context("Linked document not found")?;

    let (extracted_content, _range) = extract_header_section(
        link_data.header.as_ref().unwrap(),
        &linked_doc.references,
        slice,
    );

    match extracted_content {
        Some(content) => Ok(content.to_string()),
        None => Ok(slice.to_string()),
    }
}

/// Extracts the content header section from the provided links.
pub fn extract_header_section<'a>(
    header: &LinkHeader,
    links: &[Reference],
    content: RopeSlice<'a>,
) -> (Option<RopeSlice<'a>>, std::ops::Range<usize>) {
    let mut start_index = None;
    let mut end_index = None;

    for link in links {
        if let Reference::Header {
            level,
            content,
            span,
        } = link
        {
            if start_index.is_none() && *content == header.content && *level == header.level {
                start_index = Some(span.start);
                continue;
            } else if start_index.is_some() && *level <= header.level {
                end_index = Some(span.start);
                break;
            }
        }
    }

    match (start_index, end_index) {
        (Some(start), Some(end)) if start < end && end <= content.len_bytes() => {
            (Some(content.byte_slice(start..end)), start..end)
        }
        (Some(start), None) if start < content.len_bytes() => (
            Some(content.byte_slice(start..)),
            start..content.len_bytes(),
        ),
        _ => (None, 0..0),
    }
}
