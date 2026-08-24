mod terminal_view;

use bifur_core::fs_model::PaneState;
use bifur_core::terminal::{TerminalConfig, TerminalSession};
use gpui::prelude::*;
use gpui::*;
use gpui_platform::application;
use std::path::PathBuf;
use terminal_view::TerminalView;

struct BifurApp {
    left: PaneState,
    right: PaneState,
    active: ActiveSide,
    preview_content: String,
    terminal: Option<TerminalSession>,
    terminal_status: Option<String>,
    terminal_focused: bool,
    terminal_repaint_task: Option<Task<()>>,
    focus_handle: FocusHandle,
    left_scroll: ScrollHandle,
    right_scroll: ScrollHandle,
}

#[derive(Clone, Copy, PartialEq)]
enum ActiveSide {
    Left,
    Right,
}

fn terminal_control_byte(key: &str) -> Option<u8> {
    match key {
        "space" | "@" => Some(0x00),
        "[" => Some(0x1b),
        "\\" => Some(0x1c),
        "]" => Some(0x1d),
        "^" => Some(0x1e),
        "_" => Some(0x1f),
        "?" => Some(0x7f),
        _ if key.len() == 1 => {
            let byte = key.as_bytes()[0];
            if byte.is_ascii_alphabetic() {
                Some(byte.to_ascii_uppercase() & 0x1f)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn printable_key_char(keystroke: &Keystroke) -> Option<&str> {
    keystroke
        .key_char
        .as_deref()
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
}

fn is_altgr_printable(keystroke: &Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    if !modifiers.control || !modifiers.alt || modifiers.platform || modifiers.function {
        return false;
    }

    let Some(produced) = printable_key_char(keystroke) else {
        return false;
    };

    // AltGr is typically surfaced by GPUI as Ctrl+Alt plus a produced printable
    // character that differs from the underlying key (for example AltGr+Q -> @).
    produced != keystroke.key
}

fn control_key_identity(keystroke: &Keystroke) -> &str {
    if let Some(produced) = printable_key_char(keystroke) {
        if matches!(produced, "@" | "[" | "\\" | "]" | "^" | "_" | "?") {
            return produced;
        }
    }

    keystroke.key.as_str()
}

fn alt_meta_text(keystroke: &Keystroke) -> Option<String> {
    let produced = printable_key_char(keystroke)?;
    if produced.is_ascii() {
        return Some(produced.to_string());
    }

    // macOS Option may transform the produced glyph (Option+F -> ƒ). For
    // terminal Meta input, prefer the underlying ASCII key and preserve Shift.
    if keystroke.key.len() == 1 && keystroke.key.is_ascii() {
        let mut key = keystroke.key.clone();
        if keystroke.modifiers.shift {
            key.make_ascii_uppercase();
        }
        return Some(key);
    }

    Some(produced.to_string())
}

fn terminal_key_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    let modifiers = &keystroke.modifiers;
    if modifiers.platform || modifiers.function {
        return None;
    }

    if is_altgr_printable(keystroke) {
        return printable_key_char(keystroke).map(|text| text.as_bytes().to_vec());
    }

    if modifiers.control {
        let control = terminal_control_byte(control_key_identity(keystroke))?;
        let mut bytes = Vec::with_capacity(if modifiers.alt { 2 } else { 1 });
        if modifiers.alt {
            bytes.push(0x1b);
        }
        bytes.push(control);
        return Some(bytes);
    }

    if !modifiers.modified() {
        return match keystroke.key.as_str() {
            "enter" => Some(b"\r".to_vec()),
            "backspace" => Some(vec![0x7f]),
            "tab" => Some(b"\t".to_vec()),
            "escape" => Some(vec![0x1b]),
            "up" => Some(b"\x1b[A".to_vec()),
            "down" => Some(b"\x1b[B".to_vec()),
            "right" => Some(b"\x1b[C".to_vec()),
            "left" => Some(b"\x1b[D".to_vec()),
            _ => printable_key_char(keystroke).map(|text| text.as_bytes().to_vec()),
        };
    }

    if modifiers.alt {
        let text = alt_meta_text(keystroke)?;
        let mut bytes = Vec::with_capacity(1 + text.len());
        bytes.push(0x1b);
        bytes.extend_from_slice(text.as_bytes());
        return Some(bytes);
    }

    printable_key_char(keystroke).map(|text| text.as_bytes().to_vec())
}

impl BifurApp {
    fn new(home: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut terminal = TerminalSession::spawn(TerminalConfig {
            cwd: home.clone(),
            ..TerminalConfig::default()
        })
        .ok();

        let terminal_repaint_task = terminal
            .as_mut()
            .and_then(TerminalSession::take_event_receiver)
            .map(|event_rx| {
                cx.spawn(async move |this, cx| {
                    let mut event_rx = event_rx;
                    loop {
                        let (next_rx, event) = cx
                            .background_spawn(async move {
                                let event = event_rx.recv();
                                (event_rx, event)
                            })
                            .await;
                        event_rx = next_rx;

                        if event.is_err() {
                            break;
                        }

                        if this.update(cx, |_, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                })
            });

        Self {
            left: PaneState::new(home.clone()),
            right: PaneState::new(home),
            active: ActiveSide::Left,
            preview_content: "Select a file to preview...".to_string(),
            terminal,
            terminal_status: None,
            terminal_focused: false,
            terminal_repaint_task,
            focus_handle: cx.focus_handle(),
            left_scroll: ScrollHandle::new(),
            right_scroll: ScrollHandle::new(),
        }
    }

    fn active_pane(&self) -> &PaneState {
        match self.active {
            ActiveSide::Left => &self.left,
            ActiveSide::Right => &self.right,
        }
    }

    fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.active {
            ActiveSide::Left => &mut self.left,
            ActiveSide::Right => &mut self.right,
        }
    }

    fn active_scroll(&self) -> &ScrollHandle {
        match self.active {
            ActiveSide::Left => &self.left_scroll,
            ActiveSide::Right => &self.right_scroll,
        }
    }

    fn reveal_selection(&self) {
        self.active_scroll()
            .scroll_to_item(self.active_pane().selected);
    }

    fn sync_terminal_cwd(&mut self) {
        let cwd = self.active_pane().current_path.clone();
        self.terminal_status = self.terminal.as_mut().and_then(|terminal| {
            terminal
                .set_cwd(cwd)
                .err()
                .map(|error| format!("Terminal cwd sync failed: {error}"))
        });
    }

    fn switch_active_pane(&mut self) {
        self.active = match self.active {
            ActiveSide::Left => ActiveSide::Right,
            ActiveSide::Right => ActiveSide::Left,
        };
        self.sync_terminal_cwd();
        self.reveal_selection();
    }

    fn toggle_terminal_focus(&mut self) {
        self.terminal_focused = !self.terminal_focused;
        self.terminal_status = None;
    }

    fn send_terminal_key(&mut self, event: &KeyDownEvent) -> bool {
        let Some(bytes) = terminal_key_bytes(&event.keystroke) else {
            return false;
        };
        let Some(terminal) = &mut self.terminal else {
            self.terminal_status = Some("Terminal unavailable".to_string());
            return true;
        };

        self.terminal_status = terminal
            .send_input(&bytes)
            .err()
            .map(|error| format!("Terminal input failed: {error}"));
        true
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.key == "f6" && !event.keystroke.modifiers.modified() {
            self.toggle_terminal_focus();
            cx.stop_propagation();
            cx.notify();
            return;
        }

        if self.terminal_focused {
            if self.send_terminal_key(event) {
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }

        if event.keystroke.modifiers.modified() {
            return;
        }

        let handled = match event.keystroke.key.as_str() {
            "tab" => {
                self.switch_active_pane();
                true
            }
            "down" | "j" => {
                let changed = self.active_pane_mut().select_next();
                if changed {
                    self.reveal_selection();
                }
                changed
            }
            "up" | "k" => {
                let changed = self.active_pane_mut().select_previous();
                if changed {
                    self.reveal_selection();
                }
                changed
            }
            "enter" => {
                let changed = self.active_pane_mut().enter();
                if changed {
                    self.sync_terminal_cwd();
                    self.reveal_selection();
                }
                changed
            }
            "backspace" => {
                let changed = self.active_pane_mut().up();
                if changed {
                    self.sync_terminal_cwd();
                    self.reveal_selection();
                }
                changed
            }
            _ => false,
        };

        if handled {
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn render_pane(
        &self,
        pane: &PaneState,
        scroll: &ScrollHandle,
        is_active: bool,
        label: &str,
    ) -> Div {
        let selected = pane.selected;
        let scroll_id = if label == "LEFT" {
            "left-pane-scroll"
        } else {
            "right-pane-scroll"
        };

        div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            .when(is_active, |div| {
                div.bg(rgb(0x1a1a1a)).border_2().border_color(rgb(0x007aff))
            })
            .child(
                div()
                    .flex()
                    .justify_between()
                    .p_2()
                    .bg(rgb(0x252525))
                    .child(div().text_xs().child(format!(
                        "{} — {}",
                        label,
                        pane.current_path.display()
                    )))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child(format!("{} items", pane.entries.len())),
                    ),
            )
            .child(
                div()
                    .id(scroll_id)
                    .flex_1()
                    .overflow_scroll()
                    .track_scroll(scroll)
                    .children(pane.entries.iter().enumerate().map(|(index, entry)| {
                        let is_selected = index == selected;
                        div()
                            .flex()
                            .justify_between()
                            .px_3()
                            .py_1()
                            .text_sm()
                            .when(is_selected, |div| {
                                div.bg(rgb(0x333333)).text_color(rgb(0xffffff))
                            })
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(div().child(if entry.is_dir { "📁" } else { "📄" }))
                                    .child(entry.name.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(0x777777)).child(
                                if entry.is_dir {
                                    String::new()
                                } else {
                                    format!("{} KB", entry.size / 1024)
                                },
                            ))
                    })),
            )
    }
}

impl Focusable for BifurApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BifurApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _keep_terminal_repaint_task_alive = &self.terminal_repaint_task;
        let active = self.active;
        div()
            .id("bifur-root")
            .key_context("Bifur")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x121212))
            .text_color(rgb(0xe0e0e0))
            .child(
                div()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .px_4()
                    .bg(rgb(0x1e1e1e))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child("BIFUR — Dual Pane File Manager"),
                    )
                    .child(div().ml_auto().text_sm().text_color(rgb(0x888888)).child(
                        if self.terminal_focused {
                            "Terminal input active | Ctrl/Alt enabled | F6 pane mode"
                        } else {
                            "F6 terminal | Tab pane | ↑↓/j/k select | Enter open | Backspace up"
                        },
                    )),
            )
            .when_some(self.terminal_status.clone(), |root, status| {
                root.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .bg(rgb(0x3a241d))
                        .text_color(rgb(0xffb4a2))
                        .child(status),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.render_pane(
                        &self.left.clone(),
                        &self.left_scroll,
                        active == ActiveSide::Left,
                        "LEFT",
                    ))
                    .child(div().w(px(1.)).bg(rgb(0x2a2a2a)))
                    .child(self.render_pane(
                        &self.right.clone(),
                        &self.right_scroll,
                        active == ActiveSide::Right,
                        "RIGHT",
                    ))
                    .child(div().w(px(1.)).bg(rgb(0x2a2a2a)))
                    .child(
                        div()
                            .w(px(320.))
                            .flex()
                            .flex_col()
                            .bg(rgb(0x181818))
                            .p_3()
                            .child(div().text_sm().text_color(rgb(0x888888)).child("PREVIEW"))
                            .child(div().mt_2().text_xs().child(self.preview_content.clone()))
                            .child(
                                div()
                                    .mt_4()
                                    .p_2()
                                    .bg(if self.terminal_focused {
                                        rgb(0x1b2733)
                                    } else {
                                        rgb(0x222222)
                                    })
                                    .rounded_md()
                                    .child(match &self.terminal {
                                        Some(session) => TerminalView::render(
                                            &session.screen_snapshot(),
                                            &self.active_pane().current_path.display().to_string(),
                                        ),
                                        None => div()
                                            .text_xs()
                                            .text_color(rgb(0x888888))
                                            .child("Terminal unavailable"),
                                    }),
                            ),
                    ),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let home = dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        cx.open_window(WindowOptions::default(), |window, cx| {
            let app = cx.new(|cx| BifurApp::new(home, cx));
            app.focus_handle(cx).focus(window, cx);
            app
        })
        .expect("open BIFUR window");
        cx.activate(true);
    });
}
