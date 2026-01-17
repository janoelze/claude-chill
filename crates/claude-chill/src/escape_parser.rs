#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedEscape {
    SyncStart,
    SyncEnd,
    ClearScreen,
    ClearScrollback,
    CursorHome,
    CursorUp(u16),
    CursorCol(u16),
    ClearLine,
    Newline,
    CarriageReturn,
    Sgr(SgrCode),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgrCode {
    pub reset: bool,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub struct EscapeParser {
    state: ParserState,
    params: Vec<u16>,
    intermediate: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
    DcsString,
}

impl Default for EscapeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapeParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::with_capacity(16),
            intermediate: Vec::with_capacity(4),
        }
    }

    pub fn in_escape_sequence(&self) -> bool {
        self.state != ParserState::Ground
    }

    pub fn feed(&mut self, byte: u8) -> Option<ParsedEscape> {
        match self.state {
            ParserState::Ground => self.ground(byte),
            ParserState::Escape => self.escape(byte),
            ParserState::CsiEntry => self.csi_entry(byte),
            ParserState::CsiParam => self.csi_param(byte),
            ParserState::CsiIntermediate => self.csi_intermediate(byte),
            ParserState::OscString => self.osc_string(byte),
            ParserState::DcsString => self.dcs_string(byte),
        }
    }

    fn ground(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                None
            }
            b'\n' => Some(ParsedEscape::Newline),
            b'\r' => Some(ParsedEscape::CarriageReturn),
            _ => None,
        }
    }

    fn escape(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            b'[' => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.intermediate.clear();
                None
            }
            b']' => {
                self.state = ParserState::OscString;
                None
            }
            b'P' | b'^' | b'_' => {
                self.state = ParserState::DcsString;
                None
            }
            _ => {
                self.state = ParserState::Ground;
                Some(ParsedEscape::Other)
            }
        }
    }

    fn osc_string(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            0x07 => {
                self.state = ParserState::Ground;
                Some(ParsedEscape::Other)
            }
            0x1b => {
                self.state = ParserState::Escape;
                None
            }
            _ => None,
        }
    }

    fn dcs_string(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                None
            }
            0x9c => {
                self.state = ParserState::Ground;
                Some(ParsedEscape::Other)
            }
            _ => None,
        }
    }

    fn csi_entry(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            b'0'..=b'9' => {
                self.params.push((byte - b'0') as u16);
                self.state = ParserState::CsiParam;
                None
            }
            b';' => {
                self.params.push(0);
                self.state = ParserState::CsiParam;
                None
            }
            b'?' => {
                self.intermediate.push(byte);
                self.state = ParserState::CsiIntermediate;
                None
            }
            b'@'..=b'~' => {
                self.state = ParserState::Ground;
                self.dispatch_csi(byte)
            }
            _ => {
                self.state = ParserState::Ground;
                Some(ParsedEscape::Other)
            }
        }
    }

    fn csi_param(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            b'0'..=b'9' => {
                if let Some(last) = self.params.last_mut() {
                    *last = last.saturating_mul(10).saturating_add((byte - b'0') as u16);
                }
                None
            }
            b';' => {
                self.params.push(0);
                None
            }
            b'@'..=b'~' => {
                self.state = ParserState::Ground;
                self.dispatch_csi(byte)
            }
            _ => {
                self.state = ParserState::Ground;
                Some(ParsedEscape::Other)
            }
        }
    }

    fn csi_intermediate(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            b'0'..=b'9' => {
                if self.params.is_empty() {
                    self.params.push((byte - b'0') as u16);
                } else if let Some(last) = self.params.last_mut() {
                    *last = last.saturating_mul(10).saturating_add((byte - b'0') as u16);
                }
                None
            }
            b';' => {
                self.params.push(0);
                None
            }
            b'@'..=b'~' => {
                self.state = ParserState::Ground;
                self.dispatch_private_csi(byte)
            }
            _ => {
                self.intermediate.push(byte);
                None
            }
        }
    }

    fn dispatch_csi(&mut self, byte: u8) -> Option<ParsedEscape> {
        match byte {
            b'H' => {
                if self.params.is_empty()
                    || (self.params.len() == 2 && self.params[0] <= 1 && self.params[1] <= 1)
                {
                    Some(ParsedEscape::CursorHome)
                } else {
                    Some(ParsedEscape::Other)
                }
            }
            b'J' => {
                let param = self.params.first().copied().unwrap_or(0);
                match param {
                    2 => Some(ParsedEscape::ClearScreen),
                    3 => Some(ParsedEscape::ClearScrollback),
                    _ => Some(ParsedEscape::Other),
                }
            }
            b'A' => {
                let n = self.params.first().copied().unwrap_or(1).max(1);
                Some(ParsedEscape::CursorUp(n))
            }
            b'G' => {
                let col = self.params.first().copied().unwrap_or(1);
                Some(ParsedEscape::CursorCol(col))
            }
            b'K' => Some(ParsedEscape::ClearLine),
            b'm' => Some(ParsedEscape::Sgr(self.parse_sgr())),
            _ => Some(ParsedEscape::Other),
        }
    }

    fn dispatch_private_csi(&mut self, byte: u8) -> Option<ParsedEscape> {
        if self.intermediate.first() == Some(&b'?') {
            let param = self.params.first().copied().unwrap_or(0);
            match (param, byte) {
                (2026, b'h') => Some(ParsedEscape::SyncStart),
                (2026, b'l') => Some(ParsedEscape::SyncEnd),
                _ => Some(ParsedEscape::Other),
            }
        } else {
            Some(ParsedEscape::Other)
        }
    }

    fn parse_sgr(&self) -> SgrCode {
        let mut sgr = SgrCode::default();
        let mut i = 0;
        while i < self.params.len() {
            match self.params[i] {
                0 => sgr.reset = true,
                38 => {
                    if i + 1 < self.params.len() && self.params[i + 1] == 2 {
                        if i + 4 < self.params.len() {
                            let r = self.params[i + 2] as u8;
                            let g = self.params[i + 3] as u8;
                            let b = self.params[i + 4] as u8;
                            sgr.fg = Some(Color::Rgb(r, g, b));
                            i += 4;
                        }
                    } else if i + 1 < self.params.len()
                        && self.params[i + 1] == 5
                        && i + 2 < self.params.len()
                    {
                        sgr.fg = Some(Color::Indexed(self.params[i + 2] as u8));
                        i += 2;
                    }
                }
                48 => {
                    if i + 1 < self.params.len() && self.params[i + 1] == 2 {
                        if i + 4 < self.params.len() {
                            let r = self.params[i + 2] as u8;
                            let g = self.params[i + 3] as u8;
                            let b = self.params[i + 4] as u8;
                            sgr.bg = Some(Color::Rgb(r, g, b));
                            i += 4;
                        }
                    } else if i + 1 < self.params.len()
                        && self.params[i + 1] == 5
                        && i + 2 < self.params.len()
                    {
                        sgr.bg = Some(Color::Indexed(self.params[i + 2] as u8));
                        i += 2;
                    }
                }
                39 => sgr.fg = Some(Color::Default),
                49 => sgr.bg = Some(Color::Default),
                _ => {}
            }
            i += 1;
        }
        sgr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_sequence(bytes: &[u8]) -> Vec<ParsedEscape> {
        let mut parser = EscapeParser::new();
        bytes.iter().filter_map(|&b| parser.feed(b)).collect()
    }

    fn parse_last(bytes: &[u8]) -> Option<ParsedEscape> {
        parse_sequence(bytes).into_iter().last()
    }

    #[test]
    fn test_newline() {
        assert_eq!(parse_last(b"\n"), Some(ParsedEscape::Newline));
    }

    #[test]
    fn test_carriage_return() {
        assert_eq!(parse_last(b"\r"), Some(ParsedEscape::CarriageReturn));
    }

    #[test]
    fn test_clear_screen() {
        assert_eq!(parse_last(b"\x1b[2J"), Some(ParsedEscape::ClearScreen));
    }

    #[test]
    fn test_clear_scrollback() {
        assert_eq!(parse_last(b"\x1b[3J"), Some(ParsedEscape::ClearScrollback));
    }

    #[test]
    fn test_cursor_home() {
        assert_eq!(parse_last(b"\x1b[H"), Some(ParsedEscape::CursorHome));
        assert_eq!(parse_last(b"\x1b[1;1H"), Some(ParsedEscape::CursorHome));
    }

    #[test]
    fn test_cursor_home_with_position() {
        assert_eq!(parse_last(b"\x1b[5;10H"), Some(ParsedEscape::Other));
    }

    #[test]
    fn test_cursor_up() {
        assert_eq!(parse_last(b"\x1b[A"), Some(ParsedEscape::CursorUp(1)));
        assert_eq!(parse_last(b"\x1b[5A"), Some(ParsedEscape::CursorUp(5)));
    }

    #[test]
    fn test_cursor_col() {
        assert_eq!(parse_last(b"\x1b[G"), Some(ParsedEscape::CursorCol(1)));
        assert_eq!(parse_last(b"\x1b[15G"), Some(ParsedEscape::CursorCol(15)));
    }

    #[test]
    fn test_clear_line() {
        assert_eq!(parse_last(b"\x1b[K"), Some(ParsedEscape::ClearLine));
    }

    #[test]
    fn test_sync_start() {
        assert_eq!(parse_last(b"\x1b[?2026h"), Some(ParsedEscape::SyncStart));
    }

    #[test]
    fn test_sync_end() {
        assert_eq!(parse_last(b"\x1b[?2026l"), Some(ParsedEscape::SyncEnd));
    }

    #[test]
    fn test_sgr_reset() {
        let result = parse_last(b"\x1b[0m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: true,
                fg: None,
                bg: None,
            }))
        );
    }

    #[test]
    fn test_sgr_default_colors() {
        let result = parse_last(b"\x1b[39;49m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: false,
                fg: Some(Color::Default),
                bg: Some(Color::Default),
            }))
        );
    }

    #[test]
    fn test_sgr_indexed_fg() {
        let result = parse_last(b"\x1b[38;5;196m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: false,
                fg: Some(Color::Indexed(196)),
                bg: None,
            }))
        );
    }

    #[test]
    fn test_sgr_indexed_bg() {
        let result = parse_last(b"\x1b[48;5;21m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: false,
                fg: None,
                bg: Some(Color::Indexed(21)),
            }))
        );
    }

    #[test]
    fn test_sgr_rgb_fg() {
        let result = parse_last(b"\x1b[38;2;255;128;0m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: false,
                fg: Some(Color::Rgb(255, 128, 0)),
                bg: None,
            }))
        );
    }

    #[test]
    fn test_sgr_rgb_bg() {
        let result = parse_last(b"\x1b[48;2;0;128;255m");
        assert_eq!(
            result,
            Some(ParsedEscape::Sgr(SgrCode {
                reset: false,
                fg: None,
                bg: Some(Color::Rgb(0, 128, 255)),
            }))
        );
    }

    #[test]
    fn test_osc_title_bel_terminated() {
        let events = parse_sequence(b"\x1b]0;My Title\x07");
        assert_eq!(events, vec![ParsedEscape::Other]);
    }

    #[test]
    fn test_osc_title_st_terminated() {
        let events = parse_sequence(b"\x1b]0;My Title\x1b\\");
        assert_eq!(events, vec![ParsedEscape::Other]);
    }

    #[test]
    fn test_dcs_sequence() {
        let events = parse_sequence(b"\x1bPsome data\x1b\\");
        assert_eq!(events, vec![ParsedEscape::Other]);
    }

    #[test]
    fn test_apc_sequence() {
        let events = parse_sequence(b"\x1b_application data\x1b\\");
        assert_eq!(events, vec![ParsedEscape::Other]);
    }

    #[test]
    fn test_pm_sequence() {
        let events = parse_sequence(b"\x1b^private message\x1b\\");
        assert_eq!(events, vec![ParsedEscape::Other]);
    }

    #[test]
    fn test_mixed_content() {
        let events = parse_sequence(b"hello\nworld\r\n");
        assert_eq!(
            events,
            vec![
                ParsedEscape::Newline,
                ParsedEscape::CarriageReturn,
                ParsedEscape::Newline
            ]
        );
    }

    #[test]
    fn test_escape_followed_by_text() {
        let events = parse_sequence(b"\x1b[2Jhello\n");
        assert_eq!(
            events,
            vec![ParsedEscape::ClearScreen, ParsedEscape::Newline]
        );
    }

    #[test]
    fn test_unknown_csi() {
        assert_eq!(parse_last(b"\x1b[999z"), Some(ParsedEscape::Other));
    }
}
