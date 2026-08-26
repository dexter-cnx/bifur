#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScrollRegion {
    top: usize,
    bottom: usize,
}

impl ScrollRegion {
    pub(super) fn full(rows: usize) -> Self {
        let rows = rows.max(1);
        Self {
            top: 0,
            bottom: rows - 1,
        }
    }

    pub(super) fn from_csi_params(rows: usize, params: &str) -> Option<Self> {
        if params.bytes().any(|byte| !byte.is_ascii_digit() && byte != b';') {
            return None;
        }

        let mut parts = params.split(';');
        let top = Self::parse_csi_bound(parts.next())?;
        let bottom = Self::parse_csi_bound(parts.next())?;
        if parts.next().is_some() {
            return None;
        }

        Self::from_vt_bounds(rows, top, bottom)
    }

    fn parse_csi_bound(value: Option<&str>) -> Option<Option<usize>> {
        match value {
            None | Some("") => Some(None),
            Some(value) => value.parse::<usize>().ok().map(Some),
        }
    }

    pub(super) fn from_vt_bounds(
        rows: usize,
        top_param: Option<usize>,
        bottom_param: Option<usize>,
    ) -> Option<Self> {
        let rows = rows.max(1);
        let top = top_param.filter(|value| *value > 0).unwrap_or(1);
        let bottom = bottom_param.filter(|value| *value > 0).unwrap_or(rows);

        if top >= bottom || bottom > rows {
            return None;
        }

        Some(Self {
            top: top - 1,
            bottom: bottom - 1,
        })
    }

    pub(super) fn top(self) -> usize {
        self.top
    }

    pub(super) fn bottom(self) -> usize {
        self.bottom
    }

    pub(super) fn contains(self, row: usize) -> bool {
        (self.top..=self.bottom).contains(&row)
    }

    pub(super) fn clamp_count_from(self, row: usize, count: usize) -> usize {
        if !self.contains(row) {
            return 0;
        }

        count.min(self.bottom - row + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollRegion;

    #[test]
    fn full_region_covers_the_screen() {
        let region = ScrollRegion::full(4);

        assert_eq!(region.top(), 0);
        assert_eq!(region.bottom(), 3);
        assert!(region.contains(0));
        assert!(region.contains(3));
        assert!(!region.contains(4));
    }

    #[test]
    fn vt_bounds_use_one_based_defaults() {
        let explicit = ScrollRegion::from_vt_bounds(6, Some(2), Some(5)).unwrap();
        assert_eq!(explicit.top(), 1);
        assert_eq!(explicit.bottom(), 4);

        let defaults = ScrollRegion::from_vt_bounds(6, Some(0), Some(0)).unwrap();
        assert_eq!(defaults, ScrollRegion::full(6));
    }

    #[test]
    fn csi_params_parse_decstbm_bounds_and_defaults() {
        assert_eq!(
            ScrollRegion::from_csi_params(6, "2;5"),
            ScrollRegion::from_vt_bounds(6, Some(2), Some(5))
        );
        assert_eq!(ScrollRegion::from_csi_params(6, ""), Some(ScrollRegion::full(6)));
        assert_eq!(ScrollRegion::from_csi_params(6, ";"), Some(ScrollRegion::full(6)));
        assert_eq!(
            ScrollRegion::from_csi_params(6, "0;0"),
            Some(ScrollRegion::full(6))
        );
    }

    #[test]
    fn csi_params_reject_invalid_or_ambiguous_payloads() {
        assert!(ScrollRegion::from_csi_params(6, "4;4").is_none());
        assert!(ScrollRegion::from_csi_params(6, "1;7").is_none());
        assert!(ScrollRegion::from_csi_params(6, "1;2;3").is_none());
        assert!(ScrollRegion::from_csi_params(6, "2 r").is_none());
        assert!(ScrollRegion::from_csi_params(6, "999999999999999999999999999999").is_none());
    }

    #[test]
    fn invalid_vt_bounds_are_rejected() {
        assert!(ScrollRegion::from_vt_bounds(6, Some(4), Some(4)).is_none());
        assert!(ScrollRegion::from_vt_bounds(6, Some(5), Some(2)).is_none());
        assert!(ScrollRegion::from_vt_bounds(6, Some(1), Some(7)).is_none());
    }

    #[test]
    fn edit_counts_clamp_to_remaining_region_rows() {
        let region = ScrollRegion::from_vt_bounds(6, Some(2), Some(5)).unwrap();

        assert_eq!(region.clamp_count_from(2, 1), 1);
        assert_eq!(region.clamp_count_from(2, 99), 3);
        assert_eq!(region.clamp_count_from(0, 1), 0);
        assert_eq!(region.clamp_count_from(5, 1), 0);
    }
}
