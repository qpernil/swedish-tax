use std::panic::{catch_unwind, AssertUnwindSafe};

use swedish_tax::{
    annual_tax, annual_tax_for_income_profile, monthly_deduction, AnnualIncomeProfile, AnnualTax,
    TaxAgeGroup, TaxColumn, TaxDeduction,
};

mod plan;

/// Named status values documented by the generated C contract.
pub const SWEDISH_TAX_STATUS_OK: u32 = 0;
pub const SWEDISH_TAX_STATUS_INVALID_INPUT: u32 = 1;
pub const SWEDISH_TAX_STATUS_INTERNAL_ERROR: u32 = 2;

/// Named deduction kinds documented by the generated C contract.
pub const SWEDISH_TAX_DEDUCTION_AMOUNT: u32 = 0;
pub const SWEDISH_TAX_DEDUCTION_PERCENT: u32 = 1;

const STATUS_OK: u32 = SWEDISH_TAX_STATUS_OK;
const STATUS_INVALID_INPUT: u32 = SWEDISH_TAX_STATUS_INVALID_INPUT;
const STATUS_INTERNAL_ERROR: u32 = SWEDISH_TAX_STATUS_INTERNAL_ERROR;
const DEDUCTION_AMOUNT: u32 = SWEDISH_TAX_DEDUCTION_AMOUNT;
const DEDUCTION_PERCENT: u32 = SWEDISH_TAX_DEDUCTION_PERCENT;

/// Stable C representation of a monthly tax-table lookup.
///
/// All fields are fixed-width integers to keep the ABI straightforward for
/// Swift and other consumers. `value` is SEK when `kind` is amount, otherwise
/// it is a whole percentage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SwedishTaxDeductionResult {
    pub status: u32,
    pub kind: u32,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SwedishTaxAnnualTaxResult {
    pub status: u32,
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

impl SwedishTaxAnnualTaxResult {
    const fn error(status: u32) -> Self {
        Self {
            status,
            assessed_income: 0,
            basic_allowance: 0,
            taxable_income: 0,
            state_income_tax: 0,
            municipal_income_tax: 0,
            burial_and_religious_fee: 0,
            pension_fee: 0,
            pension_fee_credit: 0,
            work_income_credit: 0,
            sickness_compensation_credit: 0,
            earned_income_credit: 0,
            public_service_fee: 0,
            total: 0,
        }
    }

    const fn success(tax: AnnualTax) -> Self {
        Self {
            status: STATUS_OK,
            assessed_income: tax.assessed_income,
            basic_allowance: tax.basic_allowance,
            taxable_income: tax.taxable_income,
            state_income_tax: tax.state_income_tax,
            municipal_income_tax: tax.municipal_income_tax,
            burial_and_religious_fee: tax.burial_and_religious_fee,
            pension_fee: tax.pension_fee,
            pension_fee_credit: tax.pension_fee_credit,
            work_income_credit: tax.work_income_credit,
            sickness_compensation_credit: tax.sickness_compensation_credit,
            earned_income_credit: tax.earned_income_credit,
            public_service_fee: tax.public_service_fee,
            total: tax.total,
        }
    }
}

impl SwedishTaxDeductionResult {
    const fn invalid_input() -> Self {
        Self {
            status: STATUS_INVALID_INPUT,
            kind: DEDUCTION_AMOUNT,
            value: 0,
        }
    }

    const fn internal_error() -> Self {
        Self {
            status: STATUS_INTERNAL_ERROR,
            kind: DEDUCTION_AMOUNT,
            value: 0,
        }
    }
}

/// Looks up a 2026 monthly deduction using the existing Rust tax core.
///
/// `column` uses the public one-based tax-table column numbers 1 through 6.
/// No Rust panic is allowed to unwind across the C boundary.
#[no_mangle]
pub extern "C" fn swedish_tax_monthly_deduction(
    table: u32,
    column: u32,
    gross_monthly_income: u32,
) -> SwedishTaxDeductionResult {
    catch_unwind(AssertUnwindSafe(|| {
        let Ok(table) = u8::try_from(table) else {
            return SwedishTaxDeductionResult::invalid_input();
        };
        let column = match column {
            1 => TaxColumn::Column1,
            2 => TaxColumn::Column2,
            3 => TaxColumn::Column3,
            4 => TaxColumn::Column4,
            5 => TaxColumn::Column5,
            6 => TaxColumn::Column6,
            _ => return SwedishTaxDeductionResult::invalid_input(),
        };
        let Some(deduction) = monthly_deduction(table, column, gross_monthly_income) else {
            return SwedishTaxDeductionResult::invalid_input();
        };
        let (kind, value) = match deduction {
            TaxDeduction::Amount(value) => (DEDUCTION_AMOUNT, value),
            TaxDeduction::Percent(value) => (DEDUCTION_PERCENT, value),
        };
        SwedishTaxDeductionResult {
            status: STATUS_OK,
            kind,
            value,
        }
    }))
    .unwrap_or_else(|_| SwedishTaxDeductionResult::internal_error())
}

#[no_mangle]
pub extern "C" fn swedish_tax_annual_tax(
    table: u32,
    column: u32,
    gross_yearly_income: u32,
) -> SwedishTaxAnnualTaxResult {
    catch_unwind(AssertUnwindSafe(|| {
        let Some((table, column)) = validated_table_and_column(table, column) else {
            return SwedishTaxAnnualTaxResult::error(STATUS_INVALID_INPUT);
        };
        annual_tax(table, column, gross_yearly_income)
            .map(SwedishTaxAnnualTaxResult::success)
            .unwrap_or_else(|| SwedishTaxAnnualTaxResult::error(STATUS_INVALID_INPUT))
    }))
    .unwrap_or_else(|_| SwedishTaxAnnualTaxResult::error(STATUS_INTERNAL_ERROR))
}

#[no_mangle]
pub extern "C" fn swedish_tax_annual_tax_for_income_profile(
    table: u32,
    age_group: u32,
    work_income: u32,
    pension_income: u32,
) -> SwedishTaxAnnualTaxResult {
    catch_unwind(AssertUnwindSafe(|| {
        let Ok(table) = u8::try_from(table) else {
            return SwedishTaxAnnualTaxResult::error(STATUS_INVALID_INPUT);
        };
        let age_group = match age_group {
            0 => TaxAgeGroup::Under66AtYearStart,
            1 => TaxAgeGroup::AtLeast66AtYearStart,
            _ => return SwedishTaxAnnualTaxResult::error(STATUS_INVALID_INPUT),
        };
        annual_tax_for_income_profile(
            table,
            age_group,
            AnnualIncomeProfile {
                work_income,
                pension_income,
            },
        )
        .map(SwedishTaxAnnualTaxResult::success)
        .unwrap_or_else(|| SwedishTaxAnnualTaxResult::error(STATUS_INVALID_INPUT))
    }))
    .unwrap_or_else(|_| SwedishTaxAnnualTaxResult::error(STATUS_INTERNAL_ERROR))
}

fn validated_table_and_column(table: u32, column: u32) -> Option<(u8, TaxColumn)> {
    let table = u8::try_from(table).ok()?;
    let column = match column {
        1 => TaxColumn::Column1,
        2 => TaxColumn::Column2,
        3 => TaxColumn::Column3,
        4 => TaxColumn::Column4,
        5 => TaxColumn::Column5,
        6 => TaxColumn::Column6,
        _ => return None,
    };
    Some((table, column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_the_existing_amount_table() {
        assert_eq!(
            swedish_tax_monthly_deduction(32, 1, 80_000),
            SwedishTaxDeductionResult {
                status: STATUS_OK,
                kind: DEDUCTION_AMOUNT,
                value: 25_944,
            }
        );
    }

    #[test]
    fn preserves_percentage_rows() {
        assert_eq!(
            swedish_tax_monthly_deduction(32, 1, 80_001),
            SwedishTaxDeductionResult {
                status: STATUS_OK,
                kind: DEDUCTION_PERCENT,
                value: 32,
            }
        );
    }

    #[test]
    fn rejects_invalid_tables_and_columns() {
        assert_eq!(
            swedish_tax_monthly_deduction(28, 1, 50_000).status,
            STATUS_INVALID_INPUT
        );
        assert_eq!(
            swedish_tax_monthly_deduction(32, 7, 50_000).status,
            STATUS_INVALID_INPUT
        );
    }

    #[test]
    fn annual_tax_matches_the_existing_worked_example() {
        let result = swedish_tax_annual_tax(34, 1, 216_000);
        assert_eq!(result.status, STATUS_OK);
        assert_eq!(result.total, 35_889);
        assert_eq!(result.basic_allowance, 42_400);
    }

    #[test]
    fn mixed_income_uses_the_existing_profile_calculation() {
        let result = swedish_tax_annual_tax_for_income_profile(32, 0, 420_000, 120_000);
        assert_eq!(result.status, STATUS_OK);
        assert_eq!(
            result.total,
            annual_tax_for_income_profile(
                32,
                TaxAgeGroup::Under66AtYearStart,
                AnnualIncomeProfile {
                    work_income: 420_000,
                    pension_income: 120_000,
                }
            )
            .unwrap()
            .total
        );
    }
}
