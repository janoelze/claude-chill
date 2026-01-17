use crate::escape_parser::{EscapeParser, ParsedEscape};
use crate::escape_sequences::{
    PASSTHROUGH_BUFFER_CAPACITY, PENDING_ESCAPE_CAPACITY, SYNC_END, SYNC_START,
};

pub struct OutputProcessor {
    parser: EscapeParser,
    in_sync_block: bool,
    sync_buffer: Vec<u8>,
    passthrough_buffer: Vec<u8>,
    pending_escape: Vec<u8>,
}

impl Default for OutputProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputProcessor {
    pub fn new() -> Self {
        Self {
            parser: EscapeParser::new(),
            in_sync_block: false,
            sync_buffer: Vec::with_capacity(PASSTHROUGH_BUFFER_CAPACITY),
            passthrough_buffer: Vec::with_capacity(PASSTHROUGH_BUFFER_CAPACITY),
            pending_escape: Vec::with_capacity(PENDING_ESCAPE_CAPACITY),
        }
    }

    pub fn process(&mut self, data: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        self.passthrough_buffer.clear();

        for &byte in data {
            let in_escape = self.parser.in_escape_sequence();

            if in_escape && self.pending_escape.is_empty() {
                self.pending_escape.push(0x1b);
            }

            if let Some(event) = self.parser.feed(byte) {
                match event {
                    ParsedEscape::SyncStart => {
                        if !self.passthrough_buffer.is_empty() {
                            output.extend_from_slice(&self.passthrough_buffer);
                            self.passthrough_buffer.clear();
                        }
                        self.pending_escape.clear();
                        self.in_sync_block = true;
                        self.sync_buffer.clear();
                        self.sync_buffer.extend_from_slice(SYNC_START);
                        continue;
                    }
                    ParsedEscape::SyncEnd => {
                        self.pending_escape.clear();
                        if self.in_sync_block {
                            self.sync_buffer.extend_from_slice(SYNC_END);
                            output.extend_from_slice(&self.sync_buffer);
                            self.in_sync_block = false;
                        }
                        continue;
                    }
                    _ => {
                        self.flush_pending_escape();
                    }
                }
            }

            if !self.parser.in_escape_sequence() && !self.pending_escape.is_empty() {
                self.flush_pending_escape();
            }

            if self.parser.in_escape_sequence() {
                self.pending_escape.push(byte);
            } else if self.in_sync_block {
                self.sync_buffer.push(byte);
            } else {
                self.passthrough_buffer.push(byte);
            }
        }

        if !self.pending_escape.is_empty() {
            self.flush_pending_escape();
        }

        if !self.passthrough_buffer.is_empty() {
            output.extend_from_slice(&self.passthrough_buffer);
        }

        output
    }

    fn flush_pending_escape(&mut self) {
        if self.in_sync_block {
            self.sync_buffer.extend_from_slice(&self.pending_escape);
        } else {
            self.passthrough_buffer
                .extend_from_slice(&self.pending_escape);
        }
        self.pending_escape.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_no_sync() {
        let mut processor = OutputProcessor::new();
        let input = b"hello world\r\n";
        let output = processor.process(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_single_sync_block() {
        let mut processor = OutputProcessor::new();
        let input = b"\x1b[?2026hcontent\x1b[?2026l";
        let output = processor.process(input);
        assert_eq!(output, input, "Sync block should pass through unchanged");
    }

    #[test]
    fn test_sync_start_no_duplicate_byte() {
        let mut processor = OutputProcessor::new();
        let input = b"\x1b[?2026hcontent\x1b[?2026l";
        let output = processor.process(input);

        let sync_start_count = output
            .windows(SYNC_START.len())
            .filter(|w| *w == SYNC_START)
            .count();
        assert_eq!(
            sync_start_count, 1,
            "Should have exactly one SYNC_START, got {}",
            sync_start_count
        );

        let sync_end_count = output
            .windows(SYNC_END.len())
            .filter(|w| *w == SYNC_END)
            .count();
        assert_eq!(
            sync_end_count, 1,
            "Should have exactly one SYNC_END, got {}",
            sync_end_count
        );
    }

    #[test]
    fn test_content_before_sync() {
        let mut processor = OutputProcessor::new();
        let input = b"before\x1b[?2026hcontent\x1b[?2026lafter";
        let output = processor.process(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_multiple_sync_blocks() {
        let mut processor = OutputProcessor::new();
        let input = b"\x1b[?2026hblock1\x1b[?2026l\x1b[?2026hblock2\x1b[?2026l";
        let output = processor.process(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_carriage_return_preserved() {
        let mut processor = OutputProcessor::new();
        let input = b"line1\r\nline2\r\n";
        let output = processor.process(input);
        assert_eq!(output, input, "Carriage returns must be preserved");
    }

    #[test]
    fn test_carriage_return_after_sync_start() {
        let mut processor = OutputProcessor::new();
        let input = b"\x1b[?2026h\r\ncontent\x1b[?2026l";
        let output = processor.process(input);
        assert_eq!(
            output, input,
            "Carriage return after sync start must be preserved"
        );
    }

    #[test]
    fn test_carriage_return_in_sync_block() {
        let mut processor = OutputProcessor::new();
        let input = b"\x1b[?2026hline1\r\nline2\r\n\x1b[?2026l";
        let output = processor.process(input);
        assert_eq!(
            output, input,
            "Carriage returns inside sync block must be preserved"
        );
    }
}
