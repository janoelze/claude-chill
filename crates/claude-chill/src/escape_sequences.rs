pub const SYNC_START: &[u8] = b"\x1b[?2026h";
pub const SYNC_END: &[u8] = b"\x1b[?2026l";
pub const CLEAR_SCREEN: &[u8] = b"\x1b[2J";
pub const CLEAR_SCROLLBACK: &[u8] = b"\x1b[3J";
pub const CURSOR_HOME: &[u8] = b"\x1b[H";
pub const LOOKBACK_HEADER: &[u8] = b"\x1b[7m--- LOOKBACK MODE ---\x1b[0m\r\n";

pub const SYNC_BUFFER_CAPACITY: usize = 1024 * 1024;
pub const PASSTHROUGH_BUFFER_CAPACITY: usize = 65536;
pub const OUTPUT_BUFFER_CAPACITY: usize = 32768;
pub const PENDING_ESCAPE_CAPACITY: usize = 32;
pub const INPUT_BUFFER_CAPACITY: usize = 64;
