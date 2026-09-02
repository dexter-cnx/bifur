use super::{parser::Cell, scroll_region::ScrollRegion};

pub(super) fn insert_lines(
    cells: &mut [Cell],
    cols: usize,
    region: ScrollRegion,
    row: usize,
    count: usize,
    erased: Cell,
) {
    let count = region.clamp_count_from(row, count);
    if count == 0 {
        return;
    }

    let region_end = (region.bottom() + 1) * cols;
    let row_start = row * cols;
    let shift = count * cols;
    if row_start + shift < region_end {
        cells.copy_within(row_start..region_end - shift, row_start + shift);
    }
    cells[row_start..row_start + shift].fill(erased);
}

pub(super) fn delete_lines(
    cells: &mut [Cell],
    cols: usize,
    region: ScrollRegion,
    row: usize,
    count: usize,
    erased: Cell,
) {
    let count = region.clamp_count_from(row, count);
    if count == 0 {
        return;
    }

    let region_end = (region.bottom() + 1) * cols;
    let row_start = row * cols;
    let shift = count * cols;
    if row_start + shift < region_end {
        cells.copy_within(row_start + shift..region_end, row_start);
    }
    cells[region_end - shift..region_end].fill(erased);
}

pub(super) fn scroll_up(
    cells: &mut [Cell],
    cols: usize,
    region: ScrollRegion,
    count: usize,
    erased: Cell,
) {
    let rows = region.bottom() - region.top() + 1;
    let count = count.min(rows);
    if count == 0 {
        return;
    }

    let start = region.top() * cols;
    let end = (region.bottom() + 1) * cols;
    let shift = count * cols;
    if shift < end - start {
        cells.copy_within(start + shift..end, start);
    }
    cells[end - shift..end].fill(erased);
}

pub(super) fn scroll_down(
    cells: &mut [Cell],
    cols: usize,
    region: ScrollRegion,
    count: usize,
    erased: Cell,
) {
    let rows = region.bottom() - region.top() + 1;
    let count = count.min(rows);
    if count == 0 {
        return;
    }

    let start = region.top() * cols;
    let end = (region.bottom() + 1) * cols;
    let shift = count * cols;
    if shift < end - start {
        cells.copy_within(start..end - shift, start + shift);
    }
    cells[start..start + shift].fill(erased);
}

pub(super) fn scroll_up_one(cells: &mut [Cell], cols: usize, region: ScrollRegion, erased: Cell) {
    scroll_up(cells, cols, region, 1, erased);
}

#[cfg(test)]
mod tests {
    use super::{delete_lines, insert_lines, scroll_down, scroll_up, scroll_up_one};
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
    fn insert_lines_shifts_only_inside_region() {
        let mut cells = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee"]);
        let region = ScrollRegion::from_vt_bounds(5, Some(2), Some(4)).unwrap();

        insert_lines(&mut cells, 4, region, 2, 1, Cell::default());

        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "bbbb", "    ", "cccc", "eeee"]
        );
    }

    #[test]
    fn delete_lines_shifts_only_inside_region() {
        let mut cells = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee"]);
        let region = ScrollRegion::from_vt_bounds(5, Some(2), Some(4)).unwrap();

        delete_lines(&mut cells, 4, region, 2, 1, Cell::default());

        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "bbbb", "dddd", "    ", "eeee"]
        );
    }

    #[test]
    fn scroll_up_preserves_rows_outside_region() {
        let mut cells = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee"]);
        let region = ScrollRegion::from_vt_bounds(5, Some(2), Some(4)).unwrap();

        scroll_up_one(&mut cells, 4, region, Cell::default());

        assert_eq!(
            text(&cells, 4),
            vec!["aaaa", "cccc", "dddd", "    ", "eeee"]
        );
    }

    #[test]
    fn scroll_commands_support_multiple_rows_and_clamp_to_region() {
        let region = ScrollRegion::from_vt_bounds(6, Some(2), Some(5)).unwrap();
        let mut up = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee", "ffff"]);
        scroll_up(&mut up, 4, region, 2, Cell::default());
        assert_eq!(
            text(&up, 4),
            vec!["aaaa", "dddd", "eeee", "    ", "    ", "ffff"]
        );

        let mut down = cells(&["aaaa", "bbbb", "cccc", "dddd", "eeee", "ffff"]);
        scroll_down(&mut down, 4, region, 99, Cell::default());
        assert_eq!(
            text(&down, 4),
            vec!["aaaa", "    ", "    ", "    ", "    ", "ffff"]
        );
    }

    #[test]
    fn new_rows_use_supplied_erase_cell() {
        let mut cells = cells(&["aaaa", "bbbb", "cccc"]);
        let region = ScrollRegion::full(3);
        let erased = Cell {
            bg: 0x123456,
            ..Cell::default()
        };

        delete_lines(&mut cells, 4, region, 1, 99, erased);

        assert!(cells[4..]
            .iter()
            .all(|cell| cell.ch == ' ' && cell.bg == 0x123456));
    }
}
