mod input_policy;
mod terminal_view;

use bifur::pane_refresh::{PaneRefreshCoordinator, PaneRefreshRequest};
use bifur::pane_watcher::PaneSide;
use bifur_core::fs_model::PaneState;
use bifur_core::terminal::{TerminalConfig, TerminalSession};
use gpui::prelude::*;
use gpui::*;
use gpui_platform::application;
use input_policy::{translate_terminal_key, InputModifiers};
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
    pane_refresh: Option<PaneRefreshCoordinator>,
    pane_refresh_status: Option<String>,
    pane_refresh_task: Option<Task<()>>,
    focus_handle: FocusHandle,
    left_scroll: ScrollHandle,
    right_scroll: ScrollHandle,
}

#[derive(Clone, Copy, PartialEq)]
enum ActiveSide {
    Left,
    Right,
}

const TERMINAL_PANEL_WIDTH_PX: f32 = 320.0;
const TERMINAL_HORIZONTAL_CHROME_PX: f32 = 56.0;
const TERMINAL_VERTICAL_CHROME_PX: f32 = 144.0;
const TERMINAL_CELL_WIDTH_PX: f32 = 7.0;
const TERMINAL_LINE_HEIGHT_PX: f32 = 16.0;

fn terminal_size_for_window(window: &Window) -> (u16, u16) {
    let size = window.bounds().size;
    let usable_width =
        (TERMINAL_PANEL_WIDTH_PX - TERMINAL_HORIZONTAL_CHROME_PX).max(TERMINAL_CELL_WIDTH_PX);
    let usable_height =
        (f32::from(size.height) - TERMINAL_VERTICAL_CHROME_PX).max(TERMINAL_LINE_HEIGHT_PX);
    let cols = (usable_width / TERMINAL_CELL_WIDTH_PX)
        .floor()
        .clamp(1.0, u16::MAX as f32) as u16;
    let rows = (usable_height / TERMINAL_LINE_HEIGHT_PX)
        .floor()
        .clamp(1.0, u16::MAX as f32) as u16;
    (cols, rows)
}

fn terminal_key_bytes(keystroke: &Keystroke, application_cursor_keys: bool) -> Option<Vec<u8>> {
    let modifiers = &keystroke.modifiers;
    translate_terminal_key(
        keystroke.key.as_str(),
        keystroke.key_char.as_deref(),
        InputModifiers {
            shift: modifiers.shift,
            alt: modifiers.alt,
            control: modifiers.control,
            platform: modifiers.platform,
            function: modifiers.function,
        },
        application_cursor_keys,
    )
}

impl BifurApp {
    fn new(home: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (cols, rows) = terminal_size_for_window(window);
        let mut terminal = TerminalSession::spawn(TerminalConfig {
            cwd: home.clone(),
            cols,
            rows,
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

        let (mut pane_refresh, pane_refresh_status) =
            match PaneRefreshCoordinator::new(&home, &home) {
                Ok(coordinator) => (Some(coordinator), None),
                Err(error) => (None, Some(format!("Pane watcher unavailable: {error}"))),
            };

        let pane_refresh_task = pane_refresh
            .as_mut()
            .and_then(PaneRefreshCoordinator::take_receiver)
            .map(|refresh_rx| {
                cx.spawn(async move |this, cx| {
                    let mut refresh_rx = refresh_rx;
                    loop {
                        let (next_rx, side) = cx
                            .background_spawn(async move {
                                let side = refresh_rx.recv();
                                (refresh_rx, side)
                            })
                            .await;
                        refresh_rx = next_rx;

                        let Ok(side) = side else {
                            break;
                        };

                        let request = match this.update(cx, |app, _| {
                            let source_path = match side {
                                PaneSide::Left => app.left.current_path.clone(),
                                PaneSide::Right => app.right.current_path.clone(),
                            };
                            PaneRefreshRequest::new(side, source_path)
                        }) {
                            Ok(request) => request,
                            Err(_) => break,
                        };

                        let snapshot = cx.background_spawn(async move { request.read() }).await;

                        if this
                            .update(cx, |app, cx| {
                                if snapshot.apply(&mut app.left, &mut app.right) {
                                    app.reveal_selection();
                                    cx.notify();
                                }
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
            });

        cx.observe_window_bounds(window, |this, window, cx| {
            this.sync_terminal_size(window);
            cx.notify();
        })
        .detach();

        Self {
            left: PaneState::new(home.clone()),
            right: PaneState::new(home),
            active: ActiveSide::Left,
            preview_content: "Select a file to preview...".to_string(),
            terminal,
            terminal_status: None,
            terminal_focused: false,
            terminal_repaint_task,
            pane_refresh,
            pane_refresh_status,
            pane_refresh_task,
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

    fn active_pane_side(&self) -> PaneSide {
        match self.active {
            ActiveSide::Left => PaneSide::Left,
            ActiveSide::Right => PaneSide::Right,
        }
    }

    fn reveal_selection(&self) {
        self.active_scroll()
            .scroll_to_item(self.active_pane().selected);
    }

    fn sync_active_pane_watch(&mut self) {
        let side = self.active_pane_side();
        let path = self.active_pane().current_path.clone();
        let Some(pane_refresh) = &mut self.pane_refresh else {
            return;
        };

        self.pane_refresh_status = pane_refresh
            .watch_path(side, &path)
            .err()
            .map(|error| format!("Pane watcher failed: {error}"));
    }

    fn sync_terminal_size(&mut self, window: &Window) {
        let (cols, rows) = terminal_size_for_window(window);
        let Some(terminal) = &mut self.terminal else {
            return;
        };
        if terminal.config.cols == cols && terminal.config.rows == rows {
            return;
        }

        self.terminal_status = terminal
            .resize(cols, rows)
            .err()
            .map(|error| format!("Terminal resize failed: {error}"));
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
        let application_cursor_keys = self
            .terminal
            .as_ref()
            .map(TerminalSession::application_cursor_keys)
            .unwrap_or(false);
        let Some(bytes) = terminal_key_bytes(&event.keystroke, application_cursor_keys) else {
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
                    self.sync_active_pane_watch();
                    self.sync_terminal_cwd();
                    self.reveal_selection();
                }
                changed
            }
            "backspace" => {
                let changed = self.active_pane_mut().up();
                if changed {
                    self.sync_active_pane_watch();
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
        let _keep_pane_refresh_task_alive = &self.pane_refresh_task;
        let _keep_pane_refresh_watchers_alive = &self.pane_refresh;
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
            .when_some(self.pane_refresh_status.clone(), |root, status| {
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
            let app = cx.new(|cx| BifurApp::new(home, window, cx));
            app.focus_handle(cx).focus(window, cx);
            app
        })
        .expect("open BIFUR window");
        cx.activate(true);
    });
}
