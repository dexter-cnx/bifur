mod terminal_view;

use bifur_core::fs_model::PaneState;
use bifur_core::terminal::{TerminalConfig, TerminalSession};
use gpui::*;
use std::path::PathBuf;
use terminal_view::TerminalView;

struct BifurApp {
    left: PaneState,
    right: PaneState,
    active: ActiveSide,
    preview_content: String,
    terminal: Option<TerminalSession>,
    focus_handle: FocusHandle,
}

#[derive(Clone, Copy, PartialEq)]
enum ActiveSide {
    Left,
    Right,
}

impl BifurApp {
    fn new(home: PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            left: PaneState::new(home.clone()),
            right: PaneState::new(home.clone()),
            active: ActiveSide::Left,
            preview_content: "Select a file to preview...".to_string(),
            terminal: TerminalSession::spawn(TerminalConfig {
                cwd: home,
                ..TerminalConfig::default()
            })
            .ok(),
            focus_handle: cx.focus_handle(),
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

    fn sync_terminal_cwd(&mut self) {
        let cwd = self.active_pane().current_path.clone();
        if let Some(terminal) = &mut self.terminal {
            let _ = terminal.set_cwd(cwd);
        }
    }

    fn switch_active_pane(&mut self) {
        self.active = match self.active {
            ActiveSide::Left => ActiveSide::Right,
            ActiveSide::Right => ActiveSide::Left,
        };
        self.sync_terminal_cwd();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.modifiers.modified() {
            return;
        }

        let handled = match event.keystroke.key.as_str() {
            "tab" => {
                self.switch_active_pane();
                true
            }
            "down" | "j" => self.active_pane_mut().select_next(),
            "up" | "k" => self.active_pane_mut().select_previous(),
            "enter" => {
                let changed = self.active_pane_mut().enter();
                if changed {
                    self.sync_terminal_cwd();
                }
                changed
            }
            "backspace" => {
                let changed = self.active_pane_mut().up();
                if changed {
                    self.sync_terminal_cwd();
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

    fn render_pane(&self, pane: &PaneState, is_active: bool, label: &str) -> Div {
        let selected = pane.selected;
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
            .child(div().flex_1().overflow_y_scroll().children(
                pane.entries.iter().enumerate().map(|(index, entry)| {
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
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x777777))
                                .child(if entry.is_dir {
                                    String::new()
                                } else {
                                    format!("{} KB", entry.size / 1024)
                                }),
                        )
                }),
            ))
    }
}

impl Focusable for BifurApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BifurApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(
                        div()
                            .ml_auto()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child("Tab pane | ↑↓/j/k select | Enter open | Backspace up"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.render_pane(&self.left.clone(), active == ActiveSide::Left, "LEFT"))
                    .child(div().w(px(1.)).bg(rgb(0x2a2a2a)))
                    .child(self.render_pane(
                        &self.right.clone(),
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
                                div().mt_4().p_2().bg(rgb(0x222222)).rounded_md().child(
                                    match &self.terminal {
                                        Some(session) => TerminalView::render(
                                            &session.screen_snapshot(),
                                            &self.active_pane().current_path.display().to_string(),
                                        ),
                                        None => div()
                                            .text_xs()
                                            .text_color(rgb(0x888888))
                                            .child("Terminal unavailable"),
                                    },
                                ),
                            ),
                    ),
            )
    }
}

fn main() {
    App::new().run(|cx: &mut App| {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        cx.open_window(WindowOptions::default(), |window, cx| {
            let app = cx.new(|cx| BifurApp::new(home, cx));
            app.focus_handle(cx).focus(window, cx);
            app
        })
        .expect("open BIFUR window");
    });
}
