use bifur_core::terminal::ScreenBuffer;
use gpui::*;

/// GPUI renderer for the core-owned terminal surface.
///
/// It intentionally has no PTY dependency. Input and lifecycle remain owned by
/// `BifurApp`/`TerminalSession`; this type only converts a snapshot into GPUI elements.
pub struct TerminalView;

impl TerminalView {
    pub fn render(screen: &ScreenBuffer, cwd: &str) -> Div {
        let text = screen.text();
        div()
            .flex()
            .flex_col()
            .min_h(px(160.))
            .p_2()
            .bg(rgb(0x101010))
            .rounded_md()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("TERMINAL")
                    .child(cwd.to_string()),
            )
            .child(div().mt_2().text_xs().child(text))
    }
}
