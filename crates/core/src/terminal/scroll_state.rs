use super::{
    parser::Cell,
    scroll_region::ScrollRegion,
    scroll_region_ops::{delete_lines, insert_lines, scroll_up_one},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScrollState {
    region: ScrollRegion,
}

impl ScrollState {
    pub(super) fn new(rows: usize) -> Self {
        Self {
            region: ScrollRegion::full(rows),
        }
    }

    pub(super) fn reset(&mut self, rows: usize) {
        self.region = ScrollRegion::full(rows);
    }

    pub(super) fn set_from_csi(&mut self, rows: usize, params: &str) -> bool {
        let Some(region) = ScrollRegion::from_csi_params(rows, params) else {
            return false;
        };
        self.region = region;
        true
    }

    pub(super) fn insert_lines(
        self,
        cells: &mut [Cell],
        cols: usize,
        row: usize,
        count: usize,
        erased: Cell,
    ) {
        insert_lines(cells, cols, self.region, row, count, erased);
    }

    pub(super) fn delete_lines(
        self,
        cells: &mut [Cell],
        cols: usize,
        row: usize,
        count: usize,
        erased: Cell,
    ) {
        delete_lines(cells, cols, self.region, row, count, erased);
    }

    pub(super) fn scroll_up_on_bottom_margin(
        self,
        cells: &mut [Cell],
        cols: usize,
        row: usize,
        erased: Cell,
    ) -> bool {
        if row != self.region.bottom() {
            return false;
        }
        scroll_up_one(cells, cols, self.region, erased);
        true
    }

    pub(super) fn contains(self, row: usize) -> bool {
        self.region.contains(row)
    }

    #[cfg(test)]
    fn region(self) -> ScrollRegion {
        self.region
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollState;
    use crate::terminal::{parser::Cell, scroll_region::ScrollRegion};

    fn cells(lines: &[&str]) -> Vec<Cell> {
        lines
            .iter()
            .flat_map(|line| line.chars())
            .map(|ch| Cell {
                ch,
                ..Cell::default()
            })
            .collect()
    }

    fn text(cells: &[Cell], cols: usize) -> Vec<String> {
        cells
            .chunks(cols)
            .map(|row| row.iter().map(|cell| cell.ch).collect())
            .collect()
    }

    #[test]
    fn defaults_and_reset_use_full_screen_region() {
        let mut state = ScrollState::new(5);
        assert_eq!(state.region(), ScrollRegion::full(5));

        assert!(state.set_from_csi(5, "2;4"));
        assert_ne!(state.region(), ScrollRegion::full(5));

        state.reset(3);
        assert_eq!(state.region(), ScrollRegion::full(3));
    }

    #[test]
    fn invalid_margin_payload_preserves_existing_region() {
        let mut state = ScrollState::new(5);
        assert!(state.set_from_csi(5, "2;4"));
        let before = state.region();

        assert!(!state.set_from_csi(5, "4;2"));
        assert_eq!(state.region(), before);
    }

    #[test]
    fn line_operations_stay_inside_active_region() {
        let mut state = ScrollState::new(5);
        assert!(state.set_from_csi(5, "2;4"));
        let mut cells = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee"]);

        state.insert_lines(&mut cells, 4, 2, 1, Cell::default());
        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "bbbb", "    ", "cccc", "eeee"]
        );

        state.delete_lines(&mut cells, 4, 2, 1, Cell::default());
        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "bbbb", "cccc", "    ", "eeee"]
        );
    }

    #[test]
    fn lf_scrolls_only_at_bottom_margin() {
        let mut state = ScrollState::new(5);
        assert!(state.set_from_csi(5, "2;4"));
        let mut cells = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee"]);

        assert!(!state.scroll_up_on_bottom_margin(
            &mut cells,
            4,
            2,
            Cell::default()
        ));
        assert!(state.scroll_up_on_bottom_margin(
            &mut cells,
            4,
            3,
            Cell::default()
        ));
        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "cccc", "dddd", "    ", "eeee"]
        );
    }

    #[test]
    fn exposes_region_membership_for_cursor_rules() {
        let mut state = ScrollState::new(5);
        assert!(state.set_from_csi(5, "2;4"));

        assert!(!state.contains(0));
        assert!(state.contains(1));
        assert!(state.contains(3));
        assert!(!state.contains(4));
    }
}
