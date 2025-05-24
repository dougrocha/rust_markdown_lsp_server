use std::ops::Range;

use lsp_types::Position;
use ropey::RopeSlice;

pub trait TextBufferConversions {
    /// Converts a byte offset within the text buffer to an LSP-compatible Position.
    fn byte_to_lsp_position(&self, byte_offset: usize) -> Position;

    /// Converts an LSP-compatible Position to a byte offset within the text buffer.
    fn lsp_position_to_byte(&self, position: Position) -> usize;

    /// Converts a byte offset span (Range<usize>) to an LSP-compatible Range.
    fn byte_to_lsp_range(&self, span: &Range<usize>) -> lsp_types::Range {
        // Default implementation using the position converters
        // Handle empty ranges at the end of the content correctly
        if span.is_empty() && span.start == self.byte_len() {
            let pos = self.byte_to_lsp_position(span.start);
            return lsp_types::Range::new(pos, pos);
        }

        let start_pos = self.byte_to_lsp_position(span.start);
        let end_pos = self.byte_to_lsp_position(span.end);
        lsp_types::Range::new(start_pos, end_pos)
    }

    /// Converts an LSP-compatible Range to a byte offset span.
    fn lsp_range_to_byte_range(&self, range: &lsp_types::Range) -> Range<usize> {
        // Default implementation using the position converters
        let start_byte = self.lsp_position_to_byte(range.start);
        let end_byte = self.lsp_position_to_byte(range.end);
        start_byte..end_byte
    }

    /// Returns the total length of the buffer in bytes.
    fn byte_len(&self) -> usize;
}

impl TextBufferConversions for RopeSlice<'_> {
    fn byte_to_lsp_position(&self, byte_offset: usize) -> Position {
        let line_idx = self.byte_to_line(byte_offset);
        let line_char_idx = self.line_to_char(line_idx);
        let char_idx = self.byte_to_char(byte_offset) - line_char_idx;

        Position::new(line_idx as u32, char_idx as u32)
    }

    fn lsp_position_to_byte(&self, position: Position) -> usize {
        self.line_to_byte(position.line as usize) + position.character as usize
    }

    fn byte_len(&self) -> usize {
        self.len_bytes()
    }
}
