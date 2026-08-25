use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: u32,
    pub bg: u32,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: 0xE0E0E0,
            bg: 0x121212,
            bold: false,
        }
    }
}

/// UI-neutral terminal surface.
///
/// This deliberately lives in `bifur-core`: GPUI and Flutter only consume a
/// snapshot and never read PTY bytes directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenBuffer {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    cursor_col: usize,
    cursor_row: usize,
    ansi_state: AnsiState,
    csi_params: String,
    application_cursor_keys: bool,
    utf8_pending: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AnsiState {
    #[default]
    Ground,
    Escape,
    Csi,
}

impl ScreenBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            cursor_col: 0,
            cursor_row: 0,
            ansi_state: AnsiState::Ground,
            csi_params: String::new(),
            application_cursor_keys: false,
            utf8_pending: Vec::new(),
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut replacement = Self::new(cols, rows);
        let copy_rows = self.rows.min(replacement.rows);
        let copy_cols = self.cols.min(replacement.cols);
        let source_row_start = if replacement.rows < self.rows {
            self.rows - copy_rows
        } else {
            0
        };
        let target_row_start = if replacement.rows > self.rows {
            replacement.rows - copy_rows
        } else {
            0
        };

        for row_offset in 0..copy_rows {
            let source_row = source_row_start + row_offset;
            let target_row = target_row_start + row_offset;
            for col in 0..copy_cols {
                replacement.cells[target_row * replacement.cols + col] =
                    self.cells[source_row * self.cols + col].clone();
            }
        }

        replacement.cursor_col = self.cursor_col.min(replacement.cols - 1);
        replacement.cursor_row = if replacement.rows < self.rows {
            self.cursor_row
                .saturating_sub(source_row_start)
                .min(replacement.rows - 1)
        } else {
            (target_row_start + self.cursor_row).min(replacement.rows - 1)
        };
        replacement.ansi_state = self.ansi_state;
        replacement.csi_params = std::mem::take(&mut self.csi_params);
        replacement.application_cursor_keys = self.application_cursor_keys;
        replacement.utf8_pending = std::mem::take(&mut self.utf8_pending);
        *self = replacement;
    }

    pub fn application_cursor_keys(&self) -> bool {
        self.application_cursor_keys
    }

    /// Initial parser: handles normal text/control characters and a small VT100
    /// cursor subset while keeping raw escape bytes out of the visible surface.
    /// A complete VT parser can replace this without changing the public
    /// `ScreenBuffer` contract.
    ///
    /// PTY reads may split a multibyte UTF-8 code point. `utf8_pending` retains
    /// an incomplete trailing sequence until the next read instead of replacing
    /// valid Unicode with U+FFFD.
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        let mut data = std::mem::take(&mut self.utf8_pending);
        data.extend_from_slice(bytes);

        let mut offset = 0;
        while offset < data.len() {
            match std::str::from_utf8(&data[offset..]) {
                Ok(text) => {
                    for ch in text.chars() {
                        self.push_char(ch);
                    }
                    break;
                }
                Err(error) => {
                    let valid_end = offset + error.valid_up_to();
                    if valid_end > offset {
                        // SAFETY: `valid_up_to` guarantees this prefix is UTF-8.
                        let valid =
                            unsafe { std::str::from_utf8_unchecked(&data[offset..valid_end]) };
                        for ch in valid.chars() {
                            self.push_char(ch);
                        }
                    }
                    offset = valid_end;

                    match error.error_len() {
                        Some(invalid_len) => {
                            self.push_char('\u{FFFD}');
                            offset += invalid_len;
                        }
                        None => {
                            self.utf8_pending.extend_from_slice(&data[offset..]);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn lines(&self) -> Vec<String> {
        (0..self.rows)
            .map(|row| {
                let mut line = String::with_capacity(self.cols);
                for col in 0..self.cols {
                    let _ = line.write_char(self.cells[row * self.cols + col].ch);
                }
                line.trim_end().to_string()
            })
            .collect()
    }

    pub fn text(&self) -> String {
        self.lines().join("\n")
    }

    fn push_char(&mut self, ch: char) {
        match self.ansi_state {
            AnsiState::Ground => match ch {
                '\u{1b}' => self.ansi_state = AnsiState::Escape,
                '\r' => self.cursor_col = 0,
                '\n' => self.newline(),
                '\u{8}' => self.cursor_col = self.cursor_col.saturating_sub(1),
                '\t' => {
                    let spaces = 4 - (self.cursor_col % 4);
                    for _ in 0..spaces {
                        self.put_char(' ');
                    }
                }
                c if !c.is_control() => self.put_char(c),
                _ => {}
            },
            AnsiState::Escape => {
                if ch == '[' {
                    self.csi_params.clear();
                    self.ansi_state = AnsiState::Csi;
                } else {
                    self.ansi_state = AnsiState::Ground;
                }
            }
            AnsiState::Csi => {
                if ('@'..='~').contains(&ch) {
                    self.handle_csi(ch);
                    self.csi_params.clear();
                    self.ansi_state = AnsiState::Ground;
                } else {
                    self.csi_params.push(ch);
                }
            }
        }
    }

    fn handle_csi(&mut self, command: char) {
        if self.csi_params == "?1" {
            match command {
                'h' => self.application_cursor_keys = true,
                'l' => self.application_cursor_keys = false,
                _ => {}
            }
            return;
        }

        match command {
            'A' => {
                let count = self.csi_single_param(1);
                self.cursor_row = self.cursor_row.saturating_sub(count);
            }
            'B' => {
                let count = self.csi_single_param(1);
                self.cursor_row = self.cursor_row.saturating_add(count).min(self.rows - 1);
            }
            'C' => {
                let count = self.csi_single_param(1);
                self.cursor_col = self.cursor_col.saturating_add(count).min(self.cols - 1);
            }
            'D' => {
                let count = self.csi_single_param(1);
                self.cursor_col = self.cursor_col.saturating_sub(count);
            }
            'H' | 'f' => {
                let mut params = self.csi_params.split(';');
                let row = Self::csi_position_param(params.next()).saturating_sub(1);
                let col = Self::csi_position_param(params.next()).saturating_sub(1);
                self.cursor_row = row.min(self.rows - 1);
                self.cursor_col = col.min(self.cols - 1);
            }
            _ => {}
        }
    }

    fn csi_single_param(&self, default: usize) -> usize {
        self.csi_params
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    fn csi_position_param(value: Option<&str>) -> usize {
        value
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.newline();
        }
        let index = self.cursor_row * self.cols + self.cursor_col;
        self.cells[index].ch = ch;
        self.cursor_col += 1;
    }

    fn newline(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        } else {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        for row in 1..self.rows {
            for col in 0..self.cols {
                self.cells[(row - 1) * self.cols + col] = self.cells[row * self.cols + col].clone();
            }
        }
        let start = (self.rows - 1) * self.cols;
        self.cells[start..start + self.cols].fill(Cell::default());
    }
}

#[cfg(test)]
mod tests {
    use super::ScreenBuffer;

    #[test]
    fn writes_and_scrolls_text() {
        let mut screen = ScreenBuffer::new(4, 2);
        screen.push_bytes(b"one\ntwo\nthr");
        assert_eq!(screen.lines(), vec!["two", "thr"]);
    }

    #[test]
    fn removes_basic_ansi_sequence_from_visible_text() {
        let mut screen = ScreenBuffer::new(20, 2);
        screen.push_bytes(b"a\x1b[31mred\x1b[0mz");
        assert_eq!(screen.lines()[0], "aredz");
    }

    #[test]
    fn preserves_utf8_split_across_pty_reads() {
        let mut screen = ScreenBuffer::new(20, 2);
        let thai = "ก".as_bytes();
        screen.push_bytes(&thai[..1]);
        assert_eq!(screen.lines()[0], "");
        screen.push_bytes(&thai[1..]);
        assert_eq!(screen.lines()[0], "ก");
    }

    #[test]
    fn tracks_application_cursor_key_mode() {
        let mut screen = ScreenBuffer::new(20, 2);
        assert!(!screen.application_cursor_keys());

        screen.push_bytes(b"\x1b[?1h");
        assert!(screen.application_cursor_keys());

        screen.resize(40, 4);
        assert!(screen.application_cursor_keys());

        screen.push_bytes(b"\x1b[?1l");
        assert!(!screen.application_cursor_keys());
    }

    #[test]
    fn applies_relative_cursor_movement() {
        let mut screen = ScreenBuffer::new(8, 3);
        screen.push_bytes(b"abcd\x1b[2DXY");
        assert_eq!(screen.lines()[0], "abXY");

        screen.push_bytes(b"\x1b[2B\x1b[3CZ");
        assert_eq!(screen.lines()[2], "       Z");

        screen.push_bytes(b"\x1b[1A\x1b[4Dq");
        assert_eq!(screen.lines()[1], "    q");
    }

    #[test]
    fn applies_absolute_cursor_position_with_vt_defaults() {
        let mut screen = ScreenBuffer::new(8, 3);
        screen.push_bytes(b"\x1b[2;4HX\x1b[;2fY\x1b[99;99HZ");

        assert_eq!(screen.lines()[0], " Y");
        assert_eq!(screen.lines()[1], "   X");
        assert_eq!(screen.lines()[2], "       Z");
    }

    #[test]
    fn shrinking_keeps_recent_rows_and_cursor_region() {
        let mut screen = ScreenBuffer::new(8, 4);
        screen.push_bytes(b"one\ntwo\nthree\nfour");

        screen.resize(8, 2);

        assert_eq!(screen.lines(), vec!["three", "four"]);
    }

    #[test]
    fn growing_keeps_recent_rows_bottom_aligned() {
        let mut screen = ScreenBuffer::new(8, 2);
        screen.push_bytes(b"one\ntwo");

        screen.resize(8, 4);

        assert_eq!(screen.lines(), vec!["", "", "one", "two"]);
    }
}
