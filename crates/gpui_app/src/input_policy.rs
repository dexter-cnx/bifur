use bifur_core::terminal::{control_sequence, navigation_sequence, TerminalModifiers, TerminalNavigationKey};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub platform: bool,
    pub function: bool,
}

fn navigation_key(key: &str) -> Option<TerminalNavigationKey> {
    match key {
        "up" => Some(TerminalNavigationKey::Up),
        "down" => Some(TerminalNavigationKey::Down),
        "right" => Some(TerminalNavigationKey::Right),
        "left" => Some(TerminalNavigationKey::Left),
        "home" => Some(TerminalNavigationKey::Home),
        "end" => Some(TerminalNavigationKey::End),
        "insert" => Some(TerminalNavigationKey::Insert),
        "delete" => Some(TerminalNavigationKey::Delete),
        "pageup" => Some(TerminalNavigationKey::PageUp),
        "pagedown" => Some(TerminalNavigationKey::PageDown),
        _ => None,
    }
}

fn printable(key_char: Option<&str>) -> Option<&str> {
    key_char.filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
}

fn control_identity<'a>(key: &'a str, key_char: Option<&'a str>) -> &'a str {
    if let Some(produced) = printable(key_char) {
        if matches!(produced, "@" | "[" | "\\" | "]" | "^" | "_" | "?") {
            return produced;
        }
    }
    key
}

pub fn translate_terminal_key(
    key: &str,
    key_char: Option<&str>,
    modifiers: InputModifiers,
    application_cursor_keys: bool,
) -> Option<Vec<u8>> {
    if modifiers.platform {
        return None;
    }

    let navigation_key = navigation_key(key);

    if modifiers.function {
        if !modifiers.control && !modifiers.alt && !modifiers.shift {
            return navigation_key.map(|key| {
                navigation_sequence(key, application_cursor_keys, TerminalModifiers::default())
            });
        }
        return None;
    }

    let produced = printable(key_char);
    let is_altgr = modifiers.control
        && modifiers.alt
        && produced.is_some_and(|text| text != key);
    if is_altgr {
        return produced.map(|text| text.as_bytes().to_vec());
    }

    if let Some(key) = navigation_key {
        return Some(navigation_sequence(
            key,
            application_cursor_keys,
            TerminalModifiers {
                shift: modifiers.shift,
                alt: modifiers.alt,
                control: modifiers.control,
            },
        ));
    }

    if modifiers.control {
        return control_sequence(control_identity(key, key_char), modifiers.alt);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{translate_terminal_key, InputModifiers};

    #[test]
    fn preserves_altgr_printable_text() {
        assert_eq!(
            translate_terminal_key(
                "q",
                Some("@"),
                InputModifiers {
                    control: true,
                    alt: true,
                    ..InputModifiers::default()
                },
                false,
            ),
            Some(b"@".to_vec())
        );
    }

    #[test]
    fn accepts_unmodified_fn_translated_navigation() {
        assert_eq!(
            translate_terminal_key(
                "delete",
                None,
                InputModifiers {
                    function: true,
                    ..InputModifiers::default()
                },
                false,
            ),
            Some(b"\x1b[3~".to_vec())
        );
    }

    #[test]
    fn rejects_fn_navigation_with_terminal_modifiers() {
        assert_eq!(
            translate_terminal_key(
                "left",
                None,
                InputModifiers {
                    function: true,
                    control: true,
                    ..InputModifiers::default()
                },
                false,
            ),
            None
        );
    }

    #[test]
    fn rejects_platform_command_input() {
        assert_eq!(
            translate_terminal_key(
                "c",
                Some("c"),
                InputModifiers {
                    platform: true,
                    ..InputModifiers::default()
                },
                false,
            ),
            None
        );
    }

    #[test]
    fn forwards_modified_navigation_to_core_encoder() {
        assert_eq!(
            translate_terminal_key(
                "up",
                None,
                InputModifiers {
                    shift: true,
                    alt: true,
                    control: true,
                    ..InputModifiers::default()
                },
                true,
            ),
            Some(b"\x1b[1;8A".to_vec())
        );
    }

    #[test]
    fn forwards_control_punctuation_and_alt_prefix() {
        assert_eq!(
            translate_terminal_key(
                "2",
                Some("@"),
                InputModifiers {
                    control: true,
                    alt: true,
                    ..InputModifiers::default()
                },
                false,
            ),
            Some(vec![0x1b, 0x00])
        );
    }
}
