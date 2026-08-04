use std::{panic::catch_unwind, slice};

use swedish_tax::{
    AdjustmentCalibration, AppliedWithholding, Calculation, Date2026, DividendAllowanceInputs2027,
    IncomeBasisEstimate, IncomeEntry, IncomeKind, IncomePlan, PayerRole, RegularPensionPremium,
    SalaryExchange, TaxAgeGroup, TaxDeduction, VacationCompensation,
};

use super::{SwedishTaxAnnualTaxResult, STATUS_INTERNAL_ERROR, STATUS_INVALID_INPUT, STATUS_OK};

const CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxOptionalU32 {
    pub is_some: u32,
    pub value: u32,
}

impl SwedishTaxOptionalU32 {
    const fn into_option(self) -> Option<u32> {
        if self.is_some == 0 {
            None
        } else {
            Some(self.value)
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxDate {
    pub month: u32,
    pub day: u32,
}

impl SwedishTaxDate {
    fn into_core(self) -> Option<Date2026> {
        Some(Date2026::new(
            u8::try_from(self.month).ok()?,
            u8::try_from(self.day).ok()?,
        ))
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxVacationCompensation {
    pub is_some: u32,
    pub annual_entitlement_days: u32,
    pub payout_days: u32,
    pub included_in_pension_salary_basis: u32,
    pub pension_premium_override: SwedishTaxOptionalU32,
}

impl SwedishTaxVacationCompensation {
    const fn into_option(self) -> Option<VacationCompensation> {
        if self.is_some == 0 {
            None
        } else {
            Some(VacationCompensation {
                annual_entitlement_days: self.annual_entitlement_days,
                payout_days: self.payout_days,
                included_in_pension_salary_basis: self.included_in_pension_salary_basis != 0,
                pension_premium_override: self.pension_premium_override.into_option(),
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxRegularPensionPremium {
    pub is_some: u32,
    pub monthly_override: SwedishTaxOptionalU32,
}

impl SwedishTaxRegularPensionPremium {
    const fn into_option(self) -> Option<RegularPensionPremium> {
        if self.is_some == 0 {
            None
        } else {
            Some(RegularPensionPremium {
                monthly_override: self.monthly_override.into_option(),
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxSalaryExchange {
    pub is_some: u32,
    pub sacrificed_salary: u32,
    pub employer_adds_uplift: u32,
    pub uplift_basis_points: u32,
}

impl SwedishTaxSalaryExchange {
    const fn into_option(self) -> Option<SalaryExchange> {
        if self.is_some == 0 {
            None
        } else {
            Some(SalaryExchange {
                sacrificed_salary: self.sacrificed_salary,
                employer_adds_uplift: self.employer_adds_uplift != 0,
                uplift_basis_points: self.uplift_basis_points,
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxIncomeEntry {
    pub id: u64,
    pub kind: u32,
    pub amount: u32,
    pub start: SwedishTaxDate,
    pub end: SwedishTaxDate,
    pub payer_role: u32,
    pub own_company_sourced: u32,
    pub adjustment_applies: u32,
    pub use_full_year_projection_as_adjustment_basis: u32,
    pub additional_withholding_per_payment: SwedishTaxOptionalU32,
    pub actual_withholding: SwedishTaxOptionalU32,
    pub vacation_compensation: SwedishTaxVacationCompensation,
    pub regular_pension_premium: SwedishTaxRegularPensionPremium,
    pub salary_exchange: SwedishTaxSalaryExchange,
    pub included_in_pension_salary_basis: u32,
}

impl SwedishTaxIncomeEntry {
    fn into_core(self) -> Option<IncomeEntry> {
        let kind = match self.kind {
            0 => IncomeKind::AnnualSalary,
            1 => IncomeKind::MonthlySalary,
            2 => IncomeKind::OneTimeSalary,
            3 => IncomeKind::MonthlyOccupationalPension,
            4 => IncomeKind::AnnualOccupationalPension,
            5 => IncomeKind::OwnCompanyDividend,
            _ => return None,
        };
        let payer_role = match self.payer_role {
            0 => PayerRole::Main,
            1 => PayerRole::Secondary,
            _ => return None,
        };
        let mut entry = IncomeEntry::new(self.id, kind);
        entry.amount = self.amount;
        entry.start = self.start.into_core()?;
        entry.end = self.end.into_core()?;
        entry.payer_role = payer_role;
        entry.own_company_sourced = self.own_company_sourced != 0;
        entry.adjustment_applies = self.adjustment_applies != 0;
        entry.use_full_year_projection_as_adjustment_basis =
            self.use_full_year_projection_as_adjustment_basis != 0;
        entry.additional_withholding_per_payment =
            self.additional_withholding_per_payment.into_option();
        entry.actual_withholding = self.actual_withholding.into_option();
        entry.vacation_compensation = self.vacation_compensation.into_option();
        entry.regular_pension_premium = self.regular_pension_premium.into_option();
        entry.salary_exchange = self.salary_exchange.into_option();
        entry.included_in_pension_salary_basis = self.included_in_pension_salary_basis != 0;
        Some(entry)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxDividendAllowanceInputs {
    pub one_person_company: u32,
    pub ownership_basis_points: u32,
    pub other_qualified_ownership_basis_points: u32,
    pub spouse_ownership_basis_points: u32,
    pub company_cash_payroll_2026: u32,
    pub highest_related_cash_salary_2026: u32,
    pub acquisition_cost: u32,
    pub acquisition_cost_interest_basis_points: SwedishTaxOptionalU32,
    pub saved_allowance: u32,
}

impl SwedishTaxDividendAllowanceInputs {
    const fn into_core(self) -> DividendAllowanceInputs2027 {
        DividendAllowanceInputs2027 {
            one_person_company: self.one_person_company != 0,
            ownership_basis_points: self.ownership_basis_points,
            other_qualified_ownership_basis_points: self.other_qualified_ownership_basis_points,
            spouse_ownership_basis_points: self.spouse_ownership_basis_points,
            company_cash_payroll_2026: self.company_cash_payroll_2026,
            highest_related_cash_salary_2026: self.highest_related_cash_salary_2026,
            acquisition_cost: self.acquisition_cost,
            acquisition_cost_interest_basis_points: self
                .acquisition_cost_interest_basis_points
                .into_option(),
            saved_allowance: self.saved_allowance,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxPlanRequest {
    pub version: u32,
    pub table: u32,
    pub age_group: u32,
    pub entries: *const SwedishTaxIncomeEntry,
    pub entries_count: usize,
    pub adjustment_percent: SwedishTaxOptionalU32,
    pub dividend_allowance: SwedishTaxDividendAllowanceInputs,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxAdjustmentCalibration {
    pub basis_income: u32,
    pub percent: u32,
    pub formula_tax_at_basis: u32,
    pub assumed_tax_at_basis: u32,
    pub implied_tax_adjustment: i64,
    pub projected_ordinary_tax: u32,
}

impl From<AdjustmentCalibration> for SwedishTaxAdjustmentCalibration {
    fn from(value: AdjustmentCalibration) -> Self {
        Self {
            basis_income: value.basis_income,
            percent: value.percent,
            formula_tax_at_basis: value.formula_tax_at_basis,
            assumed_tax_at_basis: value.assumed_tax_at_basis,
            implied_tax_adjustment: value.implied_tax_adjustment,
            projected_ordinary_tax: value.projected_ordinary_tax,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwedishTaxWithholdingEntry {
    pub entry_id: u64,
    pub gross: u32,
    pub withheld: u32,
    pub regular_withheld: u32,
    pub supplemental_withheld: u32,
    pub additional_withheld: u32,
    pub rule_kind: u32,
    pub rule_column: u32,
    pub rule_percent: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxIncomeBasis {
    pub kind: u32,
    pub estimated_basis: u32,
    pub maximum_basis: u32,
}

impl From<IncomeBasisEstimate> for SwedishTaxIncomeBasis {
    fn from(value: IncomeBasisEstimate) -> Self {
        match value {
            IncomeBasisEstimate::Estimated(progress) => Self {
                kind: 0,
                estimated_basis: progress.estimated_basis,
                maximum_basis: progress.maximum_basis,
            },
            IncomeBasisEstimate::NotBasedOnSelectedIncome => Self {
                kind: 1,
                ..Self::default()
            },
            IncomeBasisEstimate::RequiresAdditionalInformation => Self {
                kind: 2,
                ..Self::default()
            },
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct SwedishTaxCalculationResult {
    pub status: u32,
    pub monthly_income: u32,
    pub annual_income: u32,
    pub ordinary_income: u32,
    pub work_income: u32,
    pub pension_income: u32,
    pub dividend_income: u32,
    pub sgi_annual_rate: u32,
    pub deduction_kind: u32,
    pub deduction_value: u32,
    pub annual_tax: SwedishTaxAnnualTaxResult,
    pub has_adjustment_calibration: u32,
    pub adjustment_calibration: SwedishTaxAdjustmentCalibration,
    pub ordinary_final_tax: u32,
    pub dividend_tax: u32,
    pub total_tax: u32,
    pub withholding_total: u32,
    pub withholding_entries: *mut SwedishTaxWithholdingEntry,
    pub withholding_entries_count: usize,
    pub withholding_entries_capacity: usize,
    pub withheld_tax: u32,
    pub regular_pension_premiums: u32,
    pub vacation_pension_premiums: u32,
    pub salary_exchange_sacrifice: u32,
    pub salary_exchange_pension_contributions: u32,
    pub pension_salary_basis: u32,
    pub employer_pension_contributions: u32,
    pub marginal_rate: f64,
    pub pension_progress: SwedishTaxIncomeBasis,
    pub sgi_progress: SwedishTaxIncomeBasis,
}

impl SwedishTaxCalculationResult {
    fn error(status: u32) -> Self {
        Self {
            status,
            monthly_income: 0,
            annual_income: 0,
            ordinary_income: 0,
            work_income: 0,
            pension_income: 0,
            dividend_income: 0,
            sgi_annual_rate: 0,
            deduction_kind: 0,
            deduction_value: 0,
            annual_tax: SwedishTaxAnnualTaxResult::error(status),
            has_adjustment_calibration: 0,
            adjustment_calibration: SwedishTaxAdjustmentCalibration::default(),
            ordinary_final_tax: 0,
            dividend_tax: 0,
            total_tax: 0,
            withholding_total: 0,
            withholding_entries: std::ptr::null_mut(),
            withholding_entries_count: 0,
            withholding_entries_capacity: 0,
            withheld_tax: 0,
            regular_pension_premiums: 0,
            vacation_pension_premiums: 0,
            salary_exchange_sacrifice: 0,
            salary_exchange_pension_contributions: 0,
            pension_salary_basis: 0,
            employer_pension_contributions: 0,
            marginal_rate: 0.0,
            pension_progress: SwedishTaxIncomeBasis::default(),
            sgi_progress: SwedishTaxIncomeBasis::default(),
        }
    }
}

#[no_mangle]
pub extern "C" fn swedish_tax_contract_version() -> u32 {
    CONTRACT_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn swedish_tax_calculate_plan(
    request: *const SwedishTaxPlanRequest,
) -> SwedishTaxCalculationResult {
    if request.is_null() {
        return SwedishTaxCalculationResult::error(STATUS_INVALID_INPUT);
    }
    catch_unwind(|| {
        // SAFETY: The C contract requires a readable request for this call.
        calculate(unsafe { *request })
            .unwrap_or_else(|| SwedishTaxCalculationResult::error(STATUS_INVALID_INPUT))
    })
    .unwrap_or_else(|_| SwedishTaxCalculationResult::error(STATUS_INTERNAL_ERROR))
}

fn calculate(request: SwedishTaxPlanRequest) -> Option<SwedishTaxCalculationResult> {
    if request.version != CONTRACT_VERSION
        || (request.entries.is_null() && request.entries_count != 0)
    {
        return None;
    }
    let table = u8::try_from(request.table).ok()?;
    let age_group = match request.age_group {
        0 => TaxAgeGroup::Under66AtYearStart,
        1 => TaxAgeGroup::AtLeast66AtYearStart,
        _ => return None,
    };
    let source_entries = if request.entries_count == 0 {
        &[]
    } else {
        // SAFETY: The request contract requires entries_count readable values.
        unsafe { slice::from_raw_parts(request.entries, request.entries_count) }
    };
    let entries: Option<Vec<_>> = source_entries
        .iter()
        .copied()
        .map(SwedishTaxIncomeEntry::into_core)
        .collect();
    let mut plan = IncomePlan::with_annual_salary(0);
    plan.entries = entries?;
    plan.adjustment_percent = request.adjustment_percent.into_option();
    plan.dividend_allowance = request.dividend_allowance.into_core();

    let calculation = Calculation::new(table, age_group, &plan)?;
    let withholding = plan.estimated_withholding(table, age_group);
    let mut withholding_entries: Vec<_> = withholding
        .entries
        .into_iter()
        .map(|entry| {
            let (rule_kind, rule_column, rule_percent) = match entry.rule {
                AppliedWithholding::ActualAmount => (0, 0, 0),
                AppliedWithholding::Table(column) => (1, column as u32, 0),
                AppliedWithholding::TableAndOneTime(column, percent) => (2, column as u32, percent),
                AppliedWithholding::OneTimeTable(percent) => (3, 0, percent),
                AppliedWithholding::Secondary30 => (4, 0, 0),
                AppliedWithholding::AdjustmentPercent(percent) => (5, 0, percent),
                AppliedWithholding::None => (6, 0, 0),
            };
            SwedishTaxWithholdingEntry {
                entry_id: entry.entry_id,
                gross: entry.gross,
                withheld: entry.withheld,
                regular_withheld: entry.regular_withheld,
                supplemental_withheld: entry.supplemental_withheld,
                additional_withheld: entry.additional_withheld,
                rule_kind,
                rule_column,
                rule_percent,
            }
        })
        .collect();
    let withholding_entries_pointer = withholding_entries.as_mut_ptr();
    let withholding_entries_count = withholding_entries.len();
    let withholding_entries_capacity = withholding_entries.capacity();
    std::mem::forget(withholding_entries);
    let (deduction_kind, deduction_value) = match calculation.table_deduction {
        TaxDeduction::Amount(value) => (0, value),
        TaxDeduction::Percent(value) => (1, value),
    };
    let (has_adjustment_calibration, adjustment_calibration) = calculation
        .adjustment_calibration
        .map(|value| (1, value.into()))
        .unwrap_or_default();

    Some(SwedishTaxCalculationResult {
        status: STATUS_OK,
        monthly_income: calculation.monthly_income,
        annual_income: calculation.annual_income,
        ordinary_income: calculation.ordinary_income,
        work_income: calculation.work_income,
        pension_income: calculation.pension_income,
        dividend_income: calculation.dividend_income,
        sgi_annual_rate: calculation.sgi_annual_rate,
        deduction_kind,
        deduction_value,
        annual_tax: SwedishTaxAnnualTaxResult::success(calculation.annual_tax),
        has_adjustment_calibration,
        adjustment_calibration,
        ordinary_final_tax: calculation.ordinary_final_tax,
        dividend_tax: calculation.dividend_tax,
        total_tax: calculation.total_tax,
        withholding_total: withholding.total,
        withholding_entries: withholding_entries_pointer,
        withholding_entries_count,
        withholding_entries_capacity,
        withheld_tax: calculation.withheld_tax,
        regular_pension_premiums: calculation.regular_pension_premiums,
        vacation_pension_premiums: calculation.vacation_pension_premiums,
        salary_exchange_sacrifice: calculation.salary_exchange_sacrifice,
        salary_exchange_pension_contributions: calculation.salary_exchange_pension_contributions,
        pension_salary_basis: calculation.pension_salary_basis,
        employer_pension_contributions: calculation.employer_pension_contributions,
        marginal_rate: calculation.marginal_rate,
        pension_progress: calculation.pension_progress.into(),
        sgi_progress: calculation.sgi_progress.into(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn swedish_tax_calculation_result_free(result: SwedishTaxCalculationResult) {
    if result.withholding_entries.is_null() {
        return;
    }
    // SAFETY: The array was allocated by calculate and ownership is returned
    // exactly once by the C contract.
    drop(unsafe {
        Vec::from_raw_parts(
            result.withholding_entries,
            result.withholding_entries_count,
            result.withholding_entries_capacity,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(value: u32) -> SwedishTaxOptionalU32 {
        SwedishTaxOptionalU32 { is_some: 1, value }
    }

    fn none() -> SwedishTaxOptionalU32 {
        SwedishTaxOptionalU32 {
            is_some: 0,
            value: 0,
        }
    }

    #[test]
    fn typed_plan_calls_the_complete_core() {
        let entries = [SwedishTaxIncomeEntry {
            id: 1,
            kind: 1,
            amount: 55_033,
            start: SwedishTaxDate { month: 1, day: 1 },
            end: SwedishTaxDate { month: 12, day: 31 },
            payer_role: 0,
            own_company_sourced: 0,
            adjustment_applies: 0,
            use_full_year_projection_as_adjustment_basis: 0,
            additional_withholding_per_payment: none(),
            actual_withholding: none(),
            vacation_compensation: SwedishTaxVacationCompensation {
                is_some: 0,
                annual_entitlement_days: 0,
                payout_days: 0,
                included_in_pension_salary_basis: 0,
                pension_premium_override: none(),
            },
            regular_pension_premium: SwedishTaxRegularPensionPremium {
                is_some: 1,
                monthly_override: none(),
            },
            salary_exchange: SwedishTaxSalaryExchange {
                is_some: 0,
                sacrificed_salary: 0,
                employer_adds_uplift: 0,
                uplift_basis_points: 0,
            },
            included_in_pension_salary_basis: 1,
        }];
        let request = SwedishTaxPlanRequest {
            version: CONTRACT_VERSION,
            table: 32,
            age_group: 0,
            entries: entries.as_ptr(),
            entries_count: entries.len(),
            adjustment_percent: none(),
            dividend_allowance: SwedishTaxDividendAllowanceInputs {
                one_person_company: 1,
                ownership_basis_points: 10_000,
                other_qualified_ownership_basis_points: 0,
                spouse_ownership_basis_points: 0,
                company_cash_payroll_2026: 0,
                highest_related_cash_salary_2026: 0,
                acquisition_cost: 0,
                acquisition_cost_interest_basis_points: none(),
                saved_allowance: 0,
            },
        };

        let result = unsafe { swedish_tax_calculate_plan(&request) };
        assert_eq!(result.status, STATUS_OK);
        assert_eq!(result.monthly_income, 55_033);
        assert_eq!(result.withholding_entries_count, 1);
        unsafe { swedish_tax_calculation_result_free(result) };
    }

    #[test]
    fn optional_helper_is_unambiguous() {
        assert_eq!(none().into_option(), None);
        assert_eq!(some(42).into_option(), Some(42));
    }
}
