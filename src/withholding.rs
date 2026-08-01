use crate::TaxColumn;

/// Returns the 2026 preliminary withholding rate for a one-time payment.
///
/// The applicable table is selected by the tax-table column used for the
/// recipient's main income. `annual_income` is the payer's estimated total
/// annual payment, including the one-time amount.
pub fn one_time_withholding_rate(column: TaxColumn, annual_income: u32) -> u32 {
    let thresholds: &[(u32, u32)] = match column {
        TaxColumn::Column1 => &[
            (25_041, 0),
            (82_800, 10),
            (192_000, 21),
            (477_600, 26),
            (660_000, 34),
            (u32::MAX, 54),
        ],
        TaxColumn::Column2 => &[(65_800, 0), (477_600, 26), (660_000, 34), (u32::MAX, 55)],
        TaxColumn::Column3 => &[
            (25_041, 0),
            (331_200, 10),
            (477_600, 26),
            (660_000, 34),
            (u32::MAX, 55),
        ],
        TaxColumn::Column4 => &[
            (25_041, 0),
            (54_000, 3),
            (192_000, 22),
            (660_000, 26),
            (u32::MAX, 46),
        ],
        TaxColumn::Column5 => &[
            (25_041, 0),
            (32_400, 10),
            (160_800, 29),
            (184_800, 34),
            (660_000, 38),
            (u32::MAX, 54),
        ],
        TaxColumn::Column6 => &[
            (25_041, 0),
            (160_800, 29),
            (184_800, 34),
            (660_000, 38),
            (u32::MAX, 54),
        ],
    };

    thresholds
        .iter()
        .find_map(|(maximum, rate)| (annual_income <= *maximum).then_some(*rate))
        .expect("every one-time table ends at u32::MAX")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_one_matches_the_2026_official_boundaries() {
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 25_041), 0);
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 25_042), 10);
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 82_800), 10);
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 82_801), 21);
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 660_000), 34);
        assert_eq!(one_time_withholding_rate(TaxColumn::Column1, 660_001), 54);
    }

    #[test]
    fn every_column_uses_its_high_income_rate() {
        let expected = [54, 55, 55, 46, 54, 54];
        let columns = [
            TaxColumn::Column1,
            TaxColumn::Column2,
            TaxColumn::Column3,
            TaxColumn::Column4,
            TaxColumn::Column5,
            TaxColumn::Column6,
        ];

        for (column, rate) in columns.into_iter().zip(expected) {
            assert_eq!(one_time_withholding_rate(column, u32::MAX), rate);
        }
    }
}
