use lib_core::document::references::ReferenceKind;
use lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Position, Range, SymbolKind,
};
use miette::{Context, Result};

use crate::{get_document, server_state::ServerState};

pub fn process_document_symbol(
    lsp: &mut ServerState,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let uri = params.text_document.uri;
    let document = get_document!(lsp, &uri);

    let headers: Vec<(usize, String, Range)> = document
        .references
        .iter()
        .filter_map(|r| {
            if let ReferenceKind::Header { level, content } = &r.kind {
                Some((*level, content.clone(), r.range))
            } else {
                None
            }
        })
        .collect();

    if headers.is_empty() {
        return Ok(Some(DocumentSymbolResponse::Nested(vec![])));
    }

    let total_lines = document.content.len_lines() as u32;
    let section_ranges: Vec<Range> = headers
        .iter()
        .enumerate()
        .map(|(i, (level, _, header_range))| {
            let end_line = headers[i + 1..]
                .iter()
                .find(|(next_level, _, _)| next_level <= level)
                .map(|(_, _, next_range)| next_range.start.line.saturating_sub(1))
                .unwrap_or_else(|| total_lines.saturating_sub(1));

            Range {
                start: header_range.start,
                end: Position::new(end_line, u32::MAX / 2),
            }
        })
        .collect();

    let mut idx = 0;
    let symbols = build_symbol_tree(&headers, &section_ranges, &mut idx, 0);

    Ok(Some(DocumentSymbolResponse::Nested(symbols)))
}

fn build_symbol_tree(
    headers: &[(usize, String, Range)],
    section_ranges: &[Range],
    idx: &mut usize,
    parent_level: usize,
) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    while *idx < headers.len() {
        let (level, content, header_range) = &headers[*idx];
        let level = *level;

        // Stop when we reach a header that belongs to an ancestor scope
        if parent_level != 0 && level <= parent_level {
            break;
        }

        let current_idx = *idx;
        *idx += 1;

        // Collect children: headers nested inside this one
        let children = build_symbol_tree(headers, section_ranges, idx, level);

        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: content.clone(),
            detail: None,
            kind: SymbolKind::STRING,
            tags: None,
            deprecated: None,
            range: section_ranges[current_idx],
            selection_range: *header_range,
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        };

        symbols.push(symbol);
    }

    symbols
}

#[cfg(test)]
mod tests {
    use lsp_types::{Position, Range};

    use super::*;

    fn range(start_line: u32, end_line: u32) -> Range {
        Range {
            start: Position::new(start_line, 0),
            end: Position::new(end_line, 0),
        }
    }

    fn make_headers(data: &[(usize, &str, u32)]) -> (Vec<(usize, String, Range)>, Vec<Range>) {
        let headers: Vec<(usize, String, Range)> = data
            .iter()
            .map(|(level, name, line)| (*level, name.to_string(), range(*line, *line)))
            .collect();

        let total = data.len() as u32 + 10;
        let section_ranges: Vec<Range> = headers
            .iter()
            .enumerate()
            .map(|(i, (level, _, header_range))| {
                let end_line = headers[i + 1..]
                    .iter()
                    .find(|(nl, _, _)| nl <= level)
                    .map(|(_, _, nr)| nr.start.line.saturating_sub(1))
                    .unwrap_or(total - 1);
                Range {
                    start: header_range.start,
                    end: Position::new(end_line, u32::MAX / 2),
                }
            })
            .collect();

        (headers, section_ranges)
    }

    #[test]
    fn test_flat_headers_all_same_level() {
        let data = [(1, "First", 0), (1, "Second", 5), (1, "Third", 10)];
        let (headers, section_ranges) = make_headers(&data);

        let mut idx = 0;
        let symbols = build_symbol_tree(&headers, &section_ranges, &mut idx, 0);

        assert_eq!(symbols.len(), 3);
        assert!(symbols[0].children.is_none());
        assert_eq!(symbols[0].name, "First");
        assert_eq!(symbols[1].name, "Second");
        assert_eq!(symbols[2].name, "Third");
    }

    #[test]
    fn test_nested_headers() {
        // H1 containing two H2s
        let data = [(1, "Top", 0), (2, "Sub A", 2), (2, "Sub B", 5)];
        let (headers, section_ranges) = make_headers(&data);

        let mut idx = 0;
        let symbols = build_symbol_tree(&headers, &section_ranges, &mut idx, 0);

        assert_eq!(symbols.len(), 1);
        let top = &symbols[0];
        assert_eq!(top.name, "Top");
        let children = top.children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "Sub A");
        assert_eq!(children[1].name, "Sub B");
    }

    #[test]
    fn test_deeply_nested_headers() {
        let data = [(1, "H1", 0), (2, "H2", 2), (3, "H3", 4), (2, "H2 again", 8)];
        let (headers, section_ranges) = make_headers(&data);

        let mut idx = 0;
        let symbols = build_symbol_tree(&headers, &section_ranges, &mut idx, 0);

        assert_eq!(symbols.len(), 1);
        let h1 = &symbols[0];
        let h1_children = h1.children.as_ref().unwrap();
        assert_eq!(h1_children.len(), 2);

        let h2_first = &h1_children[0];
        let h2_children = h2_first.children.as_ref().unwrap();
        assert_eq!(h2_children.len(), 1);
        assert_eq!(h2_children[0].name, "H3");

        assert!(h1_children[1].children.is_none());
    }

    #[test]
    fn test_empty_document() {
        let headers: Vec<(usize, String, Range)> = vec![];
        let section_ranges: Vec<Range> = vec![];
        let mut idx = 0;
        let symbols = build_symbol_tree(&headers, &section_ranges, &mut idx, 0);
        assert!(symbols.is_empty());
    }
}
