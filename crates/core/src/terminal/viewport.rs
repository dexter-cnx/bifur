#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalViewport {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalViewport {
    pub fn from_pixels(
        width: f32,
        height: f32,
        cell_width: f32,
        line_height: f32,
    ) -> Self {
        let cell_width = cell_width.max(f32::EPSILON);
        let line_height = line_height.max(f32::EPSILON);
        let cols = (width.max(cell_width) / cell_width)
            .floor()
            .clamp(1.0, u16::MAX as f32) as u16;
        let rows = (height.max(line_height) / line_height)
            .floor()
            .clamp(1.0, u16::MAX as f32) as u16;

        Self { cols, rows }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalViewport;

    #[test]
    fn converts_pixel_bounds_to_terminal_cells() {
        assert_eq!(
            TerminalViewport::from_pixels(264.0, 320.0, 8.0, 16.0),
            TerminalViewport { cols: 33, rows: 20 }
        );
    }

    #[test]
    fn clamps_tiny_or_invalid_metrics_to_at_least_one_cell() {
        assert_eq!(
            TerminalViewport::from_pixels(0.0, -1.0, 0.0, 0.0),
            TerminalViewport { cols: 1, rows: 1 }
        );
    }

    #[test]
    fn floors_partial_cells() {
        assert_eq!(
            TerminalViewport::from_pixels(79.9, 39.9, 8.0, 16.0),
            TerminalViewport { cols: 9, rows: 2 }
        );
    }
}
