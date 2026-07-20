//! Swedish monthly tax tables 29 through 42 for income year 2026.
//!
//! Generated from the official Skatteverket machine-readable monthly table:
//! https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5af/1765287119989/allmanna-tabeller-manad.txt
//!
//! Source SHA-256: 8c5abe81d774ce083fec81ceed430282e39208c8b5a7a961a4760e4875e850ce
//!
//! The fixed-width source format is documented by Skatteverket here:
//! https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5b8/1765287252247/postbeskrivning-allmanna-tabeller.doc
//!
//! The annual calculation follows SKV 433, edition 36 (2026):
//! https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55c/1766385913260/teknisk-beskrivning-skv-433-2026-utgava-36.pdf
//!
//! Select the column specified by the employee tax decision. Percentage
//! rows are returned as percentages so callers can apply the required
//! rounding policy explicitly.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TaxColumn {
    Column1 = 1,
    Column2 = 2,
    Column3 = 3,
    Column4 = 4,
    Column5 = 5,
    Column6 = 6,
}

impl TaxColumn {
    const fn index(self) -> usize {
        self as usize - 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaxDeduction {
    Amount(u32),
    Percent(u32),
}

/// Lowest general monthly tax table published for income year 2026.
pub const MIN_TAX_TABLE: u8 = 29;

/// Highest general monthly tax table published for income year 2026.
pub const MAX_TAX_TABLE: u8 = 42;

/// Component breakdown of the 2026 preliminary annual tax in SEK.
///
/// The calculation has the same simplifying assumptions as the tax tables:
/// unchanged income throughout the year, no other taxable income, and no
/// deductions or tax credits beyond those built into the selected column.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnnualTax {
    pub assessed_income: u32,
    pub basic_allowance: u32,
    pub taxable_income: u32,
    pub state_income_tax: u32,
    pub municipal_income_tax: u32,
    pub burial_and_religious_fee: u32,
    pub pension_fee: u32,
    pub pension_fee_credit: u32,
    pub work_income_credit: u32,
    pub sickness_compensation_credit: u32,
    pub earned_income_credit: u32,
    pub public_service_fee: u32,
    pub total: u32,
}

const PRICE_BASE_AMOUNT: u32 = 59_200;
const STATE_TAX_THRESHOLD: u32 = 643_000;
const BURIAL_AND_RELIGIOUS_RATE: u32 = 116;
const PUBLIC_SERVICE_FEE_MAXIMUM: u32 = 1_184;
const MARGINAL_INCOME_INTERVAL: u32 = 1_000;
const SCALE: i128 = 100_000_000;

/// Calculates preliminary annual tax for a yearly gross income in SEK.
///
/// Tables 29 through 42 are supported. Income is rounded down to a
/// whole hundred SEK as required by SKV 433. The selected column determines
/// which allowance, fee, and tax-credit rules apply.
pub fn annual_tax(table: u8, column: TaxColumn, gross_yearly_income: u32) -> Option<AnnualTax> {
    if !(MIN_TAX_TABLE..=MAX_TAX_TABLE).contains(&table) {
        return None;
    }

    let assessed_income = round_down_hundred(gross_yearly_income);
    let enhanced_allowance = matches!(column, TaxColumn::Column2 | TaxColumn::Column3);
    let basic_allowance = basic_allowance(assessed_income, enhanced_allowance);
    let taxable_income = assessed_income.saturating_sub(basic_allowance);

    let state_income_tax = if taxable_income >= STATE_TAX_THRESHOLD + 200 {
        (taxable_income - STATE_TAX_THRESHOLD) * 20 / 100
    } else {
        0
    };
    let municipal_rate = u32::from(table) * 100 - BURIAL_AND_RELIGIOUS_RATE;
    let municipal_income_tax = percentage_floor(taxable_income, municipal_rate, 10_000);
    let burial_and_religious_fee =
        percentage_floor(taxable_income, BURIAL_AND_RELIGIOUS_RATE, 10_000);
    let has_pension_fee = matches!(
        column,
        TaxColumn::Column1 | TaxColumn::Column3 | TaxColumn::Column5
    );
    let pension_fee = if has_pension_fee {
        pension_fee(assessed_income)
    } else {
        0
    };
    let public_service_fee = (taxable_income / 100).min(PUBLIC_SERVICE_FEE_MAXIMUM);

    // Credits are consumed in SKV 433 order. Pension-fee credit may use state
    // tax first; the remaining credits may reduce municipal income tax only.
    let pension_fee_credit = pension_fee.min(state_income_tax + municipal_income_tax);
    let pension_credit_against_municipal = pension_fee_credit.saturating_sub(state_income_tax);
    let mut municipal_tax_left =
        municipal_income_tax.saturating_sub(pension_credit_against_municipal);

    let calculated_work_credit = match column {
        TaxColumn::Column1 => {
            work_income_credit_under_66(assessed_income, basic_allowance, municipal_rate)
        }
        TaxColumn::Column3 => work_income_credit_over_66(assessed_income),
        _ => 0,
    };
    let work_income_credit = calculated_work_credit.min(municipal_tax_left);
    municipal_tax_left -= work_income_credit;

    let calculated_sickness_credit = if column == TaxColumn::Column4 {
        sickness_compensation_credit(assessed_income, basic_allowance, municipal_rate)
    } else {
        0
    };
    let sickness_compensation_credit = calculated_sickness_credit.min(municipal_tax_left);
    municipal_tax_left -= sickness_compensation_credit;

    let calculated_earned_credit = match taxable_income {
        0..=40_000 => 0,
        40_001..=240_000 => (taxable_income - 40_000) * 75 / 10_000,
        _ => 1_500,
    };
    let earned_income_credit = calculated_earned_credit.min(municipal_tax_left);

    let total = state_income_tax
        + municipal_income_tax
        + burial_and_religious_fee
        + pension_fee
        + public_service_fee
        - pension_fee_credit
        - work_income_credit
        - sickness_compensation_credit
        - earned_income_credit;

    Some(AnnualTax {
        assessed_income,
        basic_allowance,
        taxable_income,
        state_income_tax,
        municipal_income_tax,
        burial_and_religious_fee,
        pension_fee,
        pension_fee_credit,
        work_income_credit,
        sickness_compensation_credit,
        earned_income_credit,
        public_service_fee,
        total,
    })
}

/// Calculates marginal tax using monthly table withholding.
///
/// The calculation compares withholding at `monthly_income` and at 1,000 SEK
/// more, following Skatteverket's published method. At the maximum representable
/// income, the preceding 1,000 SEK interval is used instead.
pub fn marginal_rate(table: u8, column: TaxColumn, monthly_income: u32) -> Option<f64> {
    let upper_income = monthly_income.saturating_add(MARGINAL_INCOME_INTERVAL);
    let lower_income = if upper_income == monthly_income {
        monthly_income - MARGINAL_INCOME_INTERVAL
    } else {
        monthly_income
    };
    let lower_deduction = monthly_deduction(table, column, lower_income)?;
    let upper_deduction = monthly_deduction(table, column, upper_income)?;

    let tax_difference = i64::from(table_withholding(upper_income, upper_deduction))
        - i64::from(table_withholding(lower_income, lower_deduction));
    Some(tax_difference as f64 * 100.0 / f64::from(upper_income - lower_income))
}

const fn table_withholding(income: u32, deduction: TaxDeduction) -> u32 {
    match deduction {
        TaxDeduction::Amount(amount) => amount,
        TaxDeduction::Percent(percent) => (income as u64 * percent as u64 / 100) as u32,
    }
}

const fn round_down_hundred(value: u32) -> u32 {
    value / 100 * 100
}

fn basic_allowance(income: u32, enhanced: bool) -> u32 {
    let ordinary = ordinary_basic_allowance_scaled(income);
    let raw = if enhanced {
        ordinary + enhanced_basic_allowance_part_scaled(income)
    } else {
        ordinary
    };
    let limited = raw.min(scaled(income));
    round_scaled_up_to_hundred(limited)
}

fn ordinary_basic_allowance_scaled(income: u32) -> i128 {
    match income {
        0..=58_608 => pbb(423, 1_000),
        58_609..=161_024 => pbb(423, 1_000) + ratio(scaled(income - 58_608), 20, 100),
        161_025..=184_112 => pbb(77, 100),
        184_113..=466_496 => pbb(77, 100) - ratio(scaled(income - 184_112), 10, 100),
        _ => pbb(293, 1_000),
    }
}

fn enhanced_basic_allowance_part_scaled(income: u32) -> i128 {
    match income {
        0..=53_872 => pbb(687, 1_000),
        53_873..=65_712 => pbb(885, 1_000) - ratio(scaled(income), 20, 100),
        65_713..=116_328 => pbb(600, 1_000) + ratio(scaled(income), 57, 1_000),
        116_329..=161_024 => pbb(333, 1_000) + ratio(scaled(income), 1_949, 10_000),
        161_025..=184_112 => ratio(scaled(income), 3_949, 10_000) - pbb(212, 1_000),
        184_113..=191_808 => ratio(scaled(income), 4_949, 10_000) - pbb(523, 1_000),
        191_809..=296_000 => ratio(scaled(income), 356, 1_000) - pbb(73, 1_000),
        296_001..=466_496 => pbb(17, 1_000) + ratio(scaled(income), 338, 1_000),
        466_497..=478_336 => pbb(703, 1_000) + ratio(scaled(income), 251, 1_000),
        478_337..=660_672 => pbb(2_732, 1_000),
        660_673..=760_128 => pbb(9_651, 1_000) - ratio(scaled(income), 62, 100),
        _ => pbb(1_691, 1_000),
    }
}

fn pension_fee(income: u32) -> u32 {
    if income < 25_042 {
        return 0;
    }
    let raw = ratio(scaled(income.min(673_038)), 7, 100);
    (((raw + 50 * SCALE - 1) / (100 * SCALE)) * 100) as u32
}

fn work_income_credit_under_66(income: u32, allowance: u32, municipal_rate: u32) -> u32 {
    let base = match income {
        0..=53_872 => scaled(income),
        53_873..=191_808 => pbb(91, 100) + ratio(scaled(income - 53_872), 3_874, 10_000),
        191_809..=478_336 => pbb(1_813, 1_000) + ratio(scaled(income - 191_808), 251, 1_000),
        _ => pbb(3_027, 1_000),
    } - scaled(allowance);
    scaled_percentage_floor(base.max(0), municipal_rate, 10_000)
}

fn work_income_credit_over_66(income: u32) -> u32 {
    let credit = match income {
        0..=103_600 => ratio(scaled(income), 22, 100),
        103_601..=310_208 => pbb(2_635, 10_000) + ratio(scaled(income), 7, 100),
        _ => pbb(6_293, 10_000),
    };
    (credit / SCALE) as u32
}

fn sickness_compensation_credit(income: u32, allowance: u32, municipal_rate: u32) -> u32 {
    let base = match income {
        0..=53_872 => scaled(income),
        53_873..=191_808 => pbb(91, 100) + ratio(scaled(income - 53_872), 3_874, 10_000),
        _ => pbb(1_813, 1_000) + ratio(scaled(income - 191_808), 251, 1_000),
    } - scaled(allowance);
    let calculated = scaled_percentage_floor(base.max(0), municipal_rate, 10_000);
    let minimum_base = ratio(scaled(income), 45, 1_000);
    let minimum = scaled_percentage_floor(minimum_base, municipal_rate, 10_000);
    calculated.max(minimum)
}

const fn scaled(value: u32) -> i128 {
    value as i128 * SCALE
}

const fn pbb(numerator: i128, denominator: i128) -> i128 {
    PRICE_BASE_AMOUNT as i128 * SCALE * numerator / denominator
}

const fn ratio(value: i128, numerator: i128, denominator: i128) -> i128 {
    value * numerator / denominator
}

fn round_scaled_up_to_hundred(value: i128) -> u32 {
    (((value + 100 * SCALE - 1) / (100 * SCALE)) * 100) as u32
}

const fn percentage_floor(value: u32, numerator: u32, denominator: u32) -> u32 {
    (value as u64 * numerator as u64 / denominator as u64) as u32
}

fn scaled_percentage_floor(value: i128, numerator: u32, denominator: u32) -> u32 {
    (value * i128::from(numerator) / i128::from(denominator) / SCALE) as u32
}

#[derive(Clone, Copy)]
enum RowKind {
    Amount,
    Percent,
}

#[derive(Clone, Copy)]
struct Row {
    minimum: u32,
    maximum: u32,
    values: [u32; 6],
    kind: RowKind,
}

/// Looks up the withholding-tax entry for a gross monthly income in SEK.
///
/// Returns `None` for unsupported tables. Income zero has no source row but
/// naturally produces a zero deduction.
pub fn monthly_deduction(
    table: u8,
    column: TaxColumn,
    gross_monthly_income: u32,
) -> Option<TaxDeduction> {
    let rows = table_rows(table)?;
    if gross_monthly_income == 0 {
        return Some(TaxDeduction::Amount(0));
    }
    let row = rows
        .binary_search_by(|row| {
            if gross_monthly_income < row.minimum {
                std::cmp::Ordering::Greater
            } else if gross_monthly_income > row.maximum {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .ok()
        .map(|index| rows[index])?;
    let value = row.values[column.index()];
    Some(match row.kind {
        RowKind::Amount => TaxDeduction::Amount(value),
        RowKind::Percent => TaxDeduction::Percent(value),
    })
}

fn table_rows(table: u8) -> Option<&'static [Row]> {
    let index = table.checked_sub(MIN_TAX_TABLE)?;
    TABLES.get(usize::from(index)).copied()
}

const fn amount(minimum: u32, maximum: u32, values: [u32; 6]) -> Row {
    Row {
        minimum,
        maximum,
        values,
        kind: RowKind::Amount,
    }
}

const fn percent(minimum: u32, maximum: u32, values: [u32; 6]) -> Row {
    Row {
        minimum,
        maximum,
        values,
        kind: RowKind::Percent,
    }
}

include!(concat!(env!("OUT_DIR"), "/monthly_tables.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMNS: [TaxColumn; 6] = [
        TaxColumn::Column1,
        TaxColumn::Column2,
        TaxColumn::Column3,
        TaxColumn::Column4,
        TaxColumn::Column5,
        TaxColumn::Column6,
    ];

    #[test]
    fn tables_cover_every_positive_income_without_gaps() {
        assert_eq!(TABLES.iter().map(|rows| rows.len()).sum::<usize>(), 7_966);
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            let rows = table_rows(table).unwrap();
            assert_eq!(rows.first().unwrap().minimum, 1, "table {table}");
            assert_eq!(rows.last().unwrap().maximum, u32::MAX, "table {table}");
            for pair in rows.windows(2) {
                assert_eq!(pair[0].maximum + 1, pair[1].minimum, "table {table}");
            }
            for row in rows {
                for (index, column) in COLUMNS.into_iter().enumerate() {
                    let expected = Some(match row.kind {
                        RowKind::Amount => TaxDeduction::Amount(row.values[index]),
                        RowKind::Percent => TaxDeduction::Percent(row.values[index]),
                    });
                    assert_eq!(monthly_deduction(table, column, row.minimum), expected);
                    assert_eq!(monthly_deduction(table, column, row.maximum), expected);
                }
            }
        }
    }

    #[test]
    fn official_boundary_values_match_the_source_file() {
        assert_eq!(
            monthly_deduction(29, TaxColumn::Column6, 2_001),
            Some(TaxDeduction::Amount(2))
        );
        assert_eq!(
            monthly_deduction(29, TaxColumn::Column1, u32::MAX),
            Some(TaxDeduction::Percent(48))
        );
        assert_eq!(
            monthly_deduction(32, TaxColumn::Column6, 2_001),
            Some(TaxDeduction::Amount(2))
        );
        assert_eq!(
            monthly_deduction(32, TaxColumn::Column1, 80_000),
            Some(TaxDeduction::Amount(25_944))
        );
        assert_eq!(
            monthly_deduction(32, TaxColumn::Column1, 80_001),
            Some(TaxDeduction::Percent(32))
        );
        assert_eq!(
            monthly_deduction(33, TaxColumn::Column4, 80_000),
            Some(TaxDeduction::Amount(23_386))
        );
        assert_eq!(
            monthly_deduction(33, TaxColumn::Column6, 80_001),
            Some(TaxDeduction::Percent(39))
        );
        assert_eq!(
            monthly_deduction(34, TaxColumn::Column3, 80_000),
            Some(TaxDeduction::Amount(24_065))
        );
        assert_eq!(
            monthly_deduction(34, TaxColumn::Column4, u32::MAX),
            Some(TaxDeduction::Percent(45))
        );
        assert_eq!(
            monthly_deduction(42, TaxColumn::Column6, 2_001),
            Some(TaxDeduction::Amount(3))
        );
        assert_eq!(
            monthly_deduction(42, TaxColumn::Column4, u32::MAX),
            Some(TaxDeduction::Percent(51))
        );
    }

    #[test]
    fn annual_formula_matches_skv_433_worked_examples() {
        assert_eq!(
            annual_tax(34, TaxColumn::Column1, 216_000),
            Some(AnnualTax {
                assessed_income: 216_000,
                basic_allowance: 42_400,
                taxable_income: 173_600,
                state_income_tax: 0,
                municipal_income_tax: 57_010,
                burial_and_religious_fee: 2_013,
                pension_fee: 15_100,
                pension_fee_credit: 15_100,
                work_income_credit: 23_316,
                sickness_compensation_credit: 0,
                earned_income_credit: 1_002,
                public_service_fee: 1_184,
                total: 35_889,
            })
        );

        assert_eq!(
            annual_tax(34, TaxColumn::Column1, 31_200),
            Some(AnnualTax {
                assessed_income: 31_200,
                basic_allowance: 25_100,
                taxable_income: 6_100,
                state_income_tax: 0,
                municipal_income_tax: 2_003,
                burial_and_religious_fee: 70,
                pension_fee: 2_200,
                pension_fee_credit: 2_003,
                work_income_credit: 0,
                sickness_compensation_credit: 0,
                earned_income_credit: 0,
                public_service_fee: 61,
                total: 2_331,
            })
        );

        assert_eq!(
            annual_tax(34, TaxColumn::Column1, 1_020_000),
            Some(AnnualTax {
                assessed_income: 1_020_000,
                basic_allowance: 17_400,
                taxable_income: 1_002_600,
                state_income_tax: 71_920,
                municipal_income_tax: 329_253,
                burial_and_religious_fee: 11_630,
                pension_fee: 47_100,
                pension_fee_credit: 47_100,
                work_income_credit: 53_134,
                sickness_compensation_credit: 0,
                earned_income_credit: 1_500,
                public_service_fee: 1_184,
                total: 359_353,
            })
        );
    }

    #[test]
    fn annualized_formula_matches_every_monthly_amount_entry() {
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            let rows = table_rows(table).unwrap();
            for row in rows {
                if !matches!(row.kind, RowKind::Amount) {
                    continue;
                }
                let annual_income = row.maximum * 12;
                for (index, column) in COLUMNS.into_iter().enumerate() {
                    let annual = annual_tax(table, column, annual_income).unwrap();
                    assert_eq!(
                        annual.total / 12,
                        row.values[index],
                        "table {table}, column {}, monthly bracket {}..={}, annual income {}",
                        index + 1,
                        row.minimum,
                        row.maximum,
                        annual_income,
                    );
                }
            }
        }
    }

    #[test]
    fn annualized_formula_matches_every_monthly_percentage_entry() {
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            let rows = table_rows(table).unwrap();
            for row in rows {
                if !matches!(row.kind, RowKind::Percent) {
                    continue;
                }
                let first_bracket_maximum = row.minimum.div_ceil(200) * 200;
                let mut monthly_incomes = [first_bracket_maximum, row.maximum];
                if row.maximum == u32::MAX {
                    monthly_incomes[1] = first_bracket_maximum;
                }
                for monthly_income in monthly_incomes {
                    let annual_income = monthly_income * 12;
                    for (index, column) in COLUMNS.into_iter().enumerate() {
                        let annual = annual_tax(table, column, annual_income).unwrap();
                        let denominator = u64::from(annual_income);
                        let calculated = u64::from(annual.total) * 100;
                        let published = u64::from(row.values[index]) * denominator;
                        let difference = calculated.abs_diff(published);
                        assert!(
                            difference * 1_000 <= denominator * 501,
                            "table {table}, column {}, monthly income {monthly_income}",
                            index + 1,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn unsupported_tables_are_not_found_and_zero_income_has_zero_tax() {
        assert_eq!(monthly_deduction(28, TaxColumn::Column1, 50_000), None);
        assert_eq!(monthly_deduction(43, TaxColumn::Column1, 50_000), None);
        assert_eq!(
            monthly_deduction(32, TaxColumn::Column1, 0),
            Some(TaxDeduction::Amount(0))
        );
        assert_eq!(annual_tax(28, TaxColumn::Column1, 50_000), None);
        assert_eq!(annual_tax(43, TaxColumn::Column1, 50_000), None);
    }

    #[test]
    fn marginal_rate_uses_actual_table_withholding() {
        assert_eq!(marginal_rate(34, TaxColumn::Column1, 18_000), Some(25.1));

        assert_eq!(marginal_rate(29, TaxColumn::Column1, u32::MAX), Some(48.0));
        assert_eq!(marginal_rate(28, TaxColumn::Column1, 18_000), None);
    }
}
