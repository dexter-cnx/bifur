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

pub fn control_sequence(key: &str, alt: bool) -> Option<Vec<u8>> {
    let control = match key {
        "space" | "@" => 0x00,
        "[" => 0x1b,
        "\\" => 0x1c,
        "]" => 0x1d,
        "^" => 0x1e,
        "_" => 0x1f,
        "?" => 0x7f,
        _ if key.len() == 1 => {
            let byte = key.as_bytes()[0];
            if !byte.is_ascii_alphabetic() {
                return None;
            }
            byte.to_ascii_uppercase() & 0x1f
        }
        _ => return None,
    };

    let mut bytes = Vec::with_capacity(if alt { 2 } else { 1 });
    if alt {
        bytes.push(0x1b);
    }
    bytes.push(control);
    Some(bytes)
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
    use super::{control_sequence, navigation_sequence, TerminalModifiers, TerminalNavigationKey};

    #[test]
    fn encodes_ascii_control_keys() {
        assert_eq!(control_sequence("a", false), Some(vec![0x01]));
        assert_eq!(control_sequence("Z", false), Some(vec![0x1a]));
        assert_eq!(control_sequence("[", false), Some(vec![0x1b]));
        assert_eq!(control_sequence("?", false), Some(vec![0x7f]));
        assert_eq!(control_sequence("1", false), None);
    }

    #[test]
    fn prefixes_alt_control_with_escape() {
        assert_eq!(control_sequence("c", true), Some(vec![0x1b, 0x03]));
    }

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
