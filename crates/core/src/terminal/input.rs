#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalNavigationKey {
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

impl TerminalModifiers {
    fn xterm_parameter(self) -> u8 {
        1 + u8::from(self.shift) + (u8::from(self.alt) * 2) + (u8::from(self.control) * 4)
    }

    fn is_empty(self) -> bool {
        !self.shift && !self.alt && !self.control
    }
}

pub fn navigation_sequence(
    key: TerminalNavigationKey,
    application_cursor_keys: bool,
    modifiers: TerminalModifiers,
) -> Vec<u8> {
    if modifiers.is_empty() {
        let bytes: &[u8] = match key {
            TerminalNavigationKey::Up if application_cursor_keys => b"\x1bOA",
            TerminalNavigationKey::Down if application_cursor_keys => b"\x1bOB",
            TerminalNavigationKey::Right if application_cursor_keys => b"\x1bOC",
            TerminalNavigationKey::Left if application_cursor_keys => b"\x1bOD",
            TerminalNavigationKey::Home if application_cursor_keys => b"\x1bOH",
            TerminalNavigationKey::End if application_cursor_keys => b"\x1bOF",
            TerminalNavigationKey::Up => b"\x1b[A",
            TerminalNavigationKey::Down => b"\x1b[B",
            TerminalNavigationKey::Right => b"\x1b[C",
            TerminalNavigationKey::Left => b"\x1b[D",
            TerminalNavigationKey::Home => b"\x1b[H",
            TerminalNavigationKey::End => b"\x1b[F",
            TerminalNavigationKey::Insert => b"\x1b[2~",
            TerminalNavigationKey::Delete => b"\x1b[3~",
            TerminalNavigationKey::PageUp => b"\x1b[5~",
            TerminalNavigationKey::PageDown => b"\x1b[6~",
        };
        return bytes.to_vec();
    }

    let modifier = modifiers.xterm_parameter();
    let sequence = match key {
        TerminalNavigationKey::Up => format!("\x1b[1;{modifier}A"),
        TerminalNavigationKey::Down => format!("\x1b[1;{modifier}B"),
        TerminalNavigationKey::Right => format!("\x1b[1;{modifier}C"),
        TerminalNavigationKey::Left => format!("\x1b[1;{modifier}D"),
        TerminalNavigationKey::Home => format!("\x1b[1;{modifier}H"),
        TerminalNavigationKey::End => format!("\x1b[1;{modifier}F"),
        TerminalNavigationKey::Insert => format!("\x1b[2;{modifier}~"),
        TerminalNavigationKey::Delete => format!("\x1b[3;{modifier}~"),
        TerminalNavigationKey::PageUp => format!("\x1b[5;{modifier}~"),
        TerminalNavigationKey::PageDown => format!("\x1b[6;{modifier}~"),
    };
    sequence.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{navigation_sequence, TerminalModifiers, TerminalNavigationKey};

    #[test]
    fn uses_ss3_for_unmodified_application_cursor_keys() {
        assert_eq!(
            navigation_sequence(
                TerminalNavigationKey::Up,
                true,
                TerminalModifiers::default(),
            ),
            b"\x1bOA"
        );
        assert_eq!(
            navigation_sequence(
                TerminalNavigationKey::Home,
                true,
                TerminalModifiers::default(),
            ),
            b"\x1bOH"
        );
    }

    #[test]
    fn encodes_xterm_modifier_parameter_for_cursor_keys() {
        assert_eq!(
            navigation_sequence(
                TerminalNavigationKey::Left,
                true,
                TerminalModifiers {
                    shift: true,
                    alt: true,
                    control: true,
                },
            ),
            b"\x1b[1;8D"
        );
    }

    #[test]
    fn encodes_modifiers_before_tilde_for_editing_keys() {
        assert_eq!(
            navigation_sequence(
                TerminalNavigationKey::Delete,
                false,
                TerminalModifiers {
                    control: true,
                    ..TerminalModifiers::default()
                },
            ),
            b"\x1b[3;5~"
        );
        assert_eq!(
            navigation_sequence(
                TerminalNavigationKey::PageUp,
                false,
                TerminalModifiers {
                    shift: true,
                    ..TerminalModifiers::default()
                },
            ),
            b"\x1b[5;2~"
        );
    }
}
