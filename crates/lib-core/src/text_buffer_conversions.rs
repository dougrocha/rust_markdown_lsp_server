use std::ops::Range;

use lsp_types::Position;
use ropey::RopeSlice;

pub trait TextBufferConversions {
    /// Safely converts a byte offset to a Position. Returns None if out of bounds.
    fn try_byte_to_lsp_position(&self, byte_offset: usize) -> Option<Position>;

    /// Safely converts a Position to a byte offset. Returns None if out of bounds.
    fn try_lsp_position_to_byte(&self, position: Position) -> Option<usize>;

    fn byte_to_lsp_position(&self, byte_offset: usize) -> Position {
        self.try_byte_to_lsp_position(byte_offset)
            .expect("Byte offset out of bounds")
    }

    fn lsp_position_to_byte(&self, position: Position) -> usize {
        self.try_lsp_position_to_byte(position)
            .expect("LSP position out of bounds")
    }

    /// Converts a byte offset span (Range<usize>) to an LSP-compatible Range.
    fn byte_range_to_lsp_range(&self, span: &Range<usize>) -> lsp_types::Range {
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
    fn try_byte_to_lsp_position(&self, byte_offset: usize) -> Option<Position> {
        if byte_offset > self.len_bytes() {
            return None;
        }

        let line_idx = self.byte_to_line(byte_offset);

        // Safety: We already checked byte_offset bounds, so these shouldn't panic
        let line_start_char = self.line_to_char(line_idx);
        let global_char_idx = self.byte_to_char(byte_offset);

        let char_offset = global_char_idx - line_start_char;

        Some(Position::new(line_idx as u32, char_offset as u32))
    }

    fn try_lsp_position_to_byte(&self, position: Position) -> Option<usize> {
        let line_idx = position.line as usize;

        if line_idx >= self.len_lines() {
            // Edge case: LSP allows position at the very end of the file (EOF)
            // which might be on a "phantom" line if the file doesn't end in newline
            if line_idx == self.len_lines() && position.character == 0 {
                return Some(self.len_bytes());
            }
            return None;
        }

        let line_start_char = self.line_to_char(line_idx);

        let target_char_idx = line_start_char + (position.character as usize);

        if target_char_idx > self.len_chars() {
            return None;
        }

        Some(self.char_to_byte(target_char_idx))
    }

    fn byte_len(&self) -> usize {
        self.len_bytes()
    }
}
