use bifur_core::terminal::{Cell, ScreenBuffer};
use gpui::*;

/// GPUI renderer for the core-owned terminal surface.
///
/// It intentionally has no PTY dependency. Input and lifecycle remain owned by
/// `BifurApp`/`TerminalSession`; this type only converts a snapshot into GPUI elements.
pub struct TerminalView;

impl TerminalView {
    pub fn render(screen: &ScreenBuffer, cwd: &str) -> Div {
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
            .child(Self::render_screen(screen))
    }

    fn render_screen(screen: &ScreenBuffer) -> Div {
        let mut surface = div().mt_2().text_xs().flex().flex_col();

        for row in 0..screen.rows {
            let start = row * screen.cols;
            let end = start + screen.cols;
            surface = surface.child(Self::render_row(&screen.cells[start..end]));
        }

        surface
    }

    fn render_row(cells: &[Cell]) -> Div {
        let mut row = div().flex();
        let mut start = 0;

        while start < cells.len() {
            let style = (&cells[start].fg, &cells[start].bg, cells[start].bold);
            let mut end = start + 1;
            while end < cells.len()
                && cells[end].fg == *style.0
                && cells[end].bg == *style.1
                && cells[end].bold == style.2
            {
                end += 1;
            }

            let text: String = cells[start..end].iter().map(|cell| cell.ch).collect();
            let weight = if style.2 {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            };
            row = row.child(
                div()
                    .text_color(rgb(*style.0))
                    .bg(rgb(*style.1))
                    .font_weight(weight)
                    .child(text),
            );
            start = end;
        }

        row
    }
}
