#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalViewport {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalViewport {
    pub fn from_pixels(width: f32, height: f32, cell_width: f32, line_height: f32) -> Self {
        if !cell_width.is_finite()
            || !line_height.is_finite()
            || cell_width <= 0.0
            || line_height <= 0.0
        {
            return Self { cols: 1, rows: 1 };
        }

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
    fn clamps_tiny_bounds_to_at_least_one_cell() {
        assert_eq!(
            TerminalViewport::from_pixels(0.0, -1.0, 8.0, 16.0),
            TerminalViewport { cols: 1, rows: 1 }
        );
    }

    #[test]
    fn rejects_nonpositive_or_nonfinite_cell_metrics() {
        for (cell_width, line_height) in [
            (0.0, 16.0),
            (-8.0, 16.0),
            (8.0, 0.0),
            (8.0, -16.0),
            (f32::NAN, 16.0),
            (8.0, f32::INFINITY),
        ] {
            assert_eq!(
                TerminalViewport::from_pixels(800.0, 600.0, cell_width, line_height),
                TerminalViewport { cols: 1, rows: 1 }
            );
        }
    }

    #[test]
    fn floors_partial_cells() {
        assert_eq!(
            TerminalViewport::from_pixels(79.9, 39.9, 8.0, 16.0),
            TerminalViewport { cols: 9, rows: 2 }
        );
    }
}
