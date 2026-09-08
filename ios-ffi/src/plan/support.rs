//! Editor support through the shared C ABI.
use super::*;
use swedish_tax::{IncomePlanTotals, IncomePlanValidationIssue, SalaryExchangeAllowance};

/// January through December; non-monthly entries return zero in every month.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxMonthlyAmounts {
    pub january: u32,
    pub february: u32,
    pub march: u32,
    pub april: u32,
    pub may: u32,
    pub june: u32,
    pub july: u32,
    pub august: u32,
    pub september: u32,
    pub october: u32,
    pub november: u32,
    pub december: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxPlanTotals {
    pub work_income: u32,
    pub pension_income: u32,
    pub dividend_income: u32,
    pub sgi_annual_rate: u32,
    pub adjustment_basis_work_income: u32,
    pub pension_salary_basis: u32,
    pub regular_pension_premiums: u32,
    pub vacation_pension_premiums: u32,
    pub salary_exchange_sacrifice: u32,
    pub salary_exchange_pension_contributions: u32,
    pub ordinary_income: u32,
    pub monthly_taxable_income: u32,
    pub gross_income: u32,
    pub total_employer_pension_contributions: u32,
    pub employer_pension_share_of_basis: f64,
}

impl From<IncomePlanTotals> for SwedishTaxPlanTotals {
    fn from(v: IncomePlanTotals) -> Self {
        Self {
            work_income: v.work_income,
            pension_income: v.pension_income,
            dividend_income: v.dividend_income,
            sgi_annual_rate: v.sgi_annual_rate,
            adjustment_basis_work_income: v.adjustment_basis_work_income,
            pension_salary_basis: v.pension_salary_basis,
            regular_pension_premiums: v.regular_pension_premiums,
            vacation_pension_premiums: v.vacation_pension_premiums,
            salary_exchange_sacrifice: v.salary_exchange_sacrifice,
            salary_exchange_pension_contributions: v.salary_exchange_pension_contributions,
            ordinary_income: v.ordinary_income(),
            monthly_taxable_income: v.monthly_taxable_income(),
            gross_income: v.gross_income(),
            total_employer_pension_contributions: v.total_employer_pension_contributions(),
            employer_pension_share_of_basis: v.employer_pension_share_of_basis(),
        }
    }
}

/// Preview uses min(requested sacrifice, maximum_sacrifice); saved inputs and plan totals are unchanged.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxExchangeAllowance {
    pub ceiling: u32,
    pub pension_salary_basis_before: u32,
    pub pension_salary_basis_after: u32,
    pub previous_year_pension_salary_basis: SwedishTaxOptionalU32,
    pub pension_and_insurance_costs_before_exchange: SwedishTaxOptionalU32,
    pub pension_contributions_before: u32,
    pub regular_pension_premiums: u32,
    pub vacation_pension_premiums: u32,
    pub other_exchange_contributions: u32,
    pub selected_exchange_contribution: u32,
    pub total_employer_pension_contributions: u32,
    pub available_contribution: u32,
    pub maximum_sacrifice: u32,
    pub contribution_share_of_basis: f64,
}

impl From<SalaryExchangeAllowance> for SwedishTaxExchangeAllowance {
    fn from(v: SalaryExchangeAllowance) -> Self {
        Self {
            ceiling: v.ceiling,
            pension_salary_basis_before: v.pension_salary_basis_before,
            pension_salary_basis_after: v.pension_salary_basis_after,
            previous_year_pension_salary_basis: v.previous_year_pension_salary_basis.into(),
            pension_and_insurance_costs_before_exchange: v
                .pension_and_insurance_costs_before_exchange
                .into(),
            pension_contributions_before: v.pension_contributions_before,
            regular_pension_premiums: v.regular_pension_premiums,
            vacation_pension_premiums: v.vacation_pension_premiums,
            other_exchange_contributions: v.other_exchange_contributions,
            selected_exchange_contribution: v.selected_exchange_contribution,
            total_employer_pension_contributions: v.total_employer_pension_contributions,
            available_contribution: v.available_contribution,
            maximum_sacrifice: v.maximum_sacrifice,
            contribution_share_of_basis: v.contribution_share_of_basis(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxEntrySupport {
    pub status: u32,
    pub entry_id: u64,
    pub annual_amount: u32,
    pub total_annual_amount: u32,
    pub withholding_payment_count: u32,
    pub requested_additional_withholding: u32,
    pub vacation_compensation_amount: u32,
    pub regular_pension_premium_amount: u32,
    pub vacation_pension_premium_amount: u32,
    pub salary_exchange_sacrifice: u32,
    pub salary_exchange_pension_contribution: u32,
    pub pension_salary_basis_amount: u32,
    pub full_year_adjustment_basis_amount: u32,
    pub is_valid: u32,
    pub pension_benchmark_monthly: u32,
    pub suggested_vacation_days: u32,
    pub vacation_amount_per_day: f64,
    pub monthly_amounts: SwedishTaxMonthlyAmounts,
    pub has_allowance: u32,
    pub allowance: SwedishTaxExchangeAllowance,
}

impl SwedishTaxEntrySupport {
    fn from_entry(e: &IncomeEntry) -> Self {
        Self {
            status: STATUS_OK,
            entry_id: e.id,
            annual_amount: e.annual_amount(),
            total_annual_amount: e.total_annual_amount(),
            withholding_payment_count: e.withholding_payment_count(),
            requested_additional_withholding: e.requested_additional_withholding(),
            vacation_compensation_amount: e.vacation_compensation_amount(),
            regular_pension_premium_amount: e.regular_pension_premium_amount(),
            vacation_pension_premium_amount: e.vacation_pension_premium_amount(),
            salary_exchange_sacrifice: e.salary_exchange_sacrifice(),
            salary_exchange_pension_contribution: e.salary_exchange_pension_contribution(),
            pension_salary_basis_amount: e.pension_salary_basis_amount(),
            full_year_adjustment_basis_amount: e.full_year_adjustment_basis_amount(),
            is_valid: e.is_valid() as u32,
            pension_benchmark_monthly: e.regular_pension_benchmark_monthly().unwrap_or(0),
            suggested_vacation_days: VacationCompensation::suggested_days(
                e.vacation_compensation
                    .map_or(25, |v| v.annual_entitlement_days),
                e.start,
                e.end,
            ),
            vacation_amount_per_day: e
                .vacation_compensation
                .map_or(0.0, |v| v.amount_per_day(e.amount)),
            monthly_amounts: SwedishTaxMonthlyAmounts {
                january: e.amount_for_month(1),
                february: e.amount_for_month(2),
                march: e.amount_for_month(3),
                april: e.amount_for_month(4),
                may: e.amount_for_month(5),
                june: e.amount_for_month(6),
                july: e.amount_for_month(7),
                august: e.amount_for_month(8),
                september: e.amount_for_month(9),
                october: e.amount_for_month(10),
                november: e.amount_for_month(11),
                december: e.amount_for_month(12),
            },
            ..Self::default()
        }
    }
}

/// issue_kind: 0 none, 1 invalid payment period, 2 exchange exceeds allowance. ID/maximum are meaningful only for their issue.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxPlanSupport {
    pub status: u32,
    pub issue_kind: u32,
    pub issue_entry_id: u64,
    pub issue_maximum: u32,
    pub totals: SwedishTaxPlanTotals,
    pub has_uniform_monthly_table_reference: u32,
    pub salary_column: u32,
    pub pension_column: u32,
    pub entries: *mut SwedishTaxEntrySupport,
    pub entries_count: usize,
    pub entries_capacity: usize,
}

/// Editor details remain available for invalid payment periods and excessive exchanges.
/// status describes request decoding, independently of issue_kind. Totals retain requested
/// inputs (entry sacrifice is bounded by its payment); allowance previews use the permitted
/// maximum. Rows follow input order. IDs must be unique within a plan.
///
/// Ownership: request and entries must be aligned, readable and immutable until return.
/// Returned entries are Rust-owned; copy before freeing. Return the original result exactly
/// once to swedish_tax_plan_support_free, including on error. Never alter pointer/count/
/// capacity, free with another allocator, or use a returned pointer after free.
/// Build consumers against the matching generated header and Rust library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swedish_tax_plan_support(
    request: *const SwedishTaxPlanRequest,
) -> SwedishTaxPlanSupport {
    let error = |status| SwedishTaxPlanSupport {
        status,
        ..Default::default()
    };
    if request.is_null() {
        return error(STATUS_INVALID_INPUT);
    }
    catch_unwind(|| {
        // SAFETY: caller provides a readable request and input array for this call.
        let Some((_, age, plan)) = plan_from_request(unsafe { *request }) else {
            return error(STATUS_INVALID_INPUT);
        };
        let mut ids = std::collections::HashSet::new();
        if plan.entries.iter().any(|e| !ids.insert(e.id)) {
            return error(STATUS_INVALID_INPUT);
        }
        let (issue_kind, issue_entry_id, issue_maximum) = match plan.validation_issue() {
            None => (0, 0, 0),
            Some(IncomePlanValidationIssue::InvalidPaymentPeriod { entry_id }) => (1, entry_id, 0),
            Some(IncomePlanValidationIssue::SalaryExchangeExceedsAllowance {
                entry_id,
                maximum,
            }) => (2, entry_id, maximum),
        };
        let mut entries: Vec<_> = plan
            .entries
            .iter()
            .map(|e| {
                let mut row = SwedishTaxEntrySupport::from_entry(e);
                if let Some(allowance) = plan.salary_exchange_allowance(e.id) {
                    row.has_allowance = 1;
                    row.allowance = allowance.into();
                }
                row
            })
            .collect();
        let result = SwedishTaxPlanSupport {
            status: STATUS_OK,
            issue_kind,
            issue_entry_id,
            issue_maximum,
            totals: plan.totals().into(),
            has_uniform_monthly_table_reference: plan.has_uniform_monthly_table_reference() as u32,
            salary_column: age.salary_column() as u32,
            pension_column: age.pension_column() as u32,
            entries: if entries.is_empty() {
                std::ptr::null_mut()
            } else {
                entries.as_mut_ptr()
            },
            entries_count: entries.len(),
            entries_capacity: entries.capacity(),
        };
        std::mem::forget(entries);
        result
    })
    .unwrap_or_else(|_| error(STATUS_INTERNAL_ERROR))
}

/// Returns the original support allocation to Rust exactly once. A zero/error result is safe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swedish_tax_plan_support_free(result: SwedishTaxPlanSupport) {
    if !result.entries.is_null() {
        // SAFETY: caller returns the unmodified allocation exactly once.
        drop(unsafe {
            Vec::from_raw_parts(
                result.entries,
                result.entries_count,
                result.entries_capacity,
            )
        });
    }
}

/// Allocation-free entry preview for editors. No plan-level allowance is returned.
/// A null or undecodable entry returns INVALID_INPUT. The input is borrowed until return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swedish_tax_entry_support(
    entry: *const SwedishTaxIncomeEntry,
) -> SwedishTaxEntrySupport {
    let error = |status| SwedishTaxEntrySupport {
        status,
        ..Default::default()
    };
    if entry.is_null() {
        return error(STATUS_INVALID_INPUT);
    }
    catch_unwind(|| {
        // SAFETY: caller provides a readable entry.
        unsafe { *entry }
            .into_core()
            .as_ref()
            .map(SwedishTaxEntrySupport::from_entry)
            .unwrap_or_else(|| error(STATUS_INVALID_INPUT))
    })
    .unwrap_or_else(|_| error(STATUS_INTERNAL_ERROR))
}

/// Policy defaults for new editor values. Existing saved values remain explicit inputs.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SwedishTaxPlanningPolicy {
    pub regular_pension_monthly_threshold: u32,
    pub default_vacation_rate_basis_points: u32,
    pub default_exchange_uplift_basis_points: u32,
    pub employer_pension_allowance_maximum: u32,
    pub acquisition_cost_threshold: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn swedish_tax_planning_policy() -> SwedishTaxPlanningPolicy {
    SwedishTaxPlanningPolicy {
        regular_pension_monthly_threshold: swedish_tax::REGULAR_PENSION_MONTHLY_THRESHOLD,
        default_vacation_rate_basis_points:
            swedish_tax::DEFAULT_VACATION_COMPENSATION_RATE_BASIS_POINTS,
        default_exchange_uplift_basis_points:
            swedish_tax::DEFAULT_SALARY_EXCHANGE_UPLIFT_BASIS_POINTS,
        employer_pension_allowance_maximum: swedish_tax::EMPLOYER_PENSION_ALLOWANCE_MAXIMUM,
        acquisition_cost_threshold: swedish_tax::DIVIDEND_ACQUISITION_COST_THRESHOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: u64, kind: u32, amount: u32) -> SwedishTaxIncomeEntry {
        // Every input field is an integer or an integer aggregate; zero is valid.
        let mut e: SwedishTaxIncomeEntry = unsafe { std::mem::zeroed() };
        e.id = id;
        e.kind = kind;
        e.amount = amount;
        e.start = SwedishTaxDate { month: 1, day: 1 };
        e.end = SwedishTaxDate { month: 12, day: 31 };
        e.included_in_pension_salary_basis = 1;
        e.regular_pension_premium.is_some = 1;
        e
    }
    fn request(entries: &[SwedishTaxIncomeEntry]) -> SwedishTaxPlanRequest {
        // Null pointers are valid with zero length; replaced before reading the request.
        let mut r: SwedishTaxPlanRequest = unsafe { std::mem::zeroed() };
        r.table = 32;
        r.entries = entries.as_ptr();
        r.entries_count = entries.len();
        r
    }
    #[test]
    fn row_previews_match_core_across_kinds_periods_and_extremes() {
        for kind in 0..6 {
            for amount in [0, 1, 52_125, 93_000, u32::MAX] {
                for annual_daily in [0, 1] {
                    let mut e = entry(u64::MAX, kind, amount);
                    e.start = SwedishTaxDate { month: 3, day: 15 };
                    e.end = SwedishTaxDate { month: 10, day: 18 };
                    e.use_annual_daily_rate_for_partial_months = annual_daily;
                    e.additional_withholding_per_payment = Some(u32::MAX).into();
                    e.vacation_compensation = SwedishTaxVacationCompensation {
                        is_some: 1,
                        annual_entitlement_days: 30,
                        payout_days: 20,
                        rate_basis_points: 540,
                        included_in_pension_salary_basis: 1,
                        pension_premium_override: None.into(),
                    };
                    e.salary_exchange = SwedishTaxSalaryExchange {
                        is_some: 1,
                        sacrificed_salary: amount,
                        employer_adds_uplift: 1,
                        uplift_basis_points: 576,
                        previous_year_pension_salary_basis: None.into(),
                        pension_and_insurance_costs_before_exchange: None.into(),
                    };
                    let core = e.into_core().unwrap();
                    let row = unsafe { swedish_tax_entry_support(&e) };
                    assert_eq!(row.status, STATUS_OK);
                    assert_eq!(row.entry_id, u64::MAX);
                    assert_eq!(row.has_allowance, 0);
                    assert_eq!(row.annual_amount, core.annual_amount());
                    assert_eq!(row.total_annual_amount, core.total_annual_amount());
                    assert_eq!(
                        row.withholding_payment_count,
                        core.withholding_payment_count()
                    );
                    assert_eq!(
                        row.requested_additional_withholding,
                        core.requested_additional_withholding()
                    );
                    assert_eq!(
                        row.vacation_compensation_amount,
                        core.vacation_compensation_amount()
                    );
                    assert_eq!(
                        row.regular_pension_premium_amount,
                        core.regular_pension_premium_amount()
                    );
                    assert_eq!(
                        row.vacation_pension_premium_amount,
                        core.vacation_pension_premium_amount()
                    );
                    assert_eq!(
                        row.salary_exchange_sacrifice,
                        core.salary_exchange_sacrifice()
                    );
                    assert_eq!(
                        row.salary_exchange_pension_contribution,
                        core.salary_exchange_pension_contribution()
                    );
                    assert_eq!(
                        row.pension_salary_basis_amount,
                        core.pension_salary_basis_amount()
                    );
                    assert_eq!(
                        row.full_year_adjustment_basis_amount,
                        core.full_year_adjustment_basis_amount()
                    );
                    assert_eq!(row.monthly_amounts.january, core.amount_for_month(1));
                    assert_eq!(row.monthly_amounts.february, core.amount_for_month(2));
                    assert_eq!(row.monthly_amounts.march, core.amount_for_month(3));
                    assert_eq!(row.monthly_amounts.april, core.amount_for_month(4));
                    assert_eq!(row.monthly_amounts.may, core.amount_for_month(5));
                    assert_eq!(row.monthly_amounts.june, core.amount_for_month(6));
                    assert_eq!(row.monthly_amounts.july, core.amount_for_month(7));
                    assert_eq!(row.monthly_amounts.august, core.amount_for_month(8));
                    assert_eq!(row.monthly_amounts.september, core.amount_for_month(9));
                    assert_eq!(row.monthly_amounts.october, core.amount_for_month(10));
                    assert_eq!(row.monthly_amounts.november, core.amount_for_month(11));
                    assert_eq!(row.monthly_amounts.december, core.amount_for_month(12));
                    assert_eq!(
                        row.pension_benchmark_monthly,
                        core.regular_pension_benchmark_monthly().unwrap_or(0)
                    );
                    assert_eq!(
                        row.suggested_vacation_days,
                        VacationCompensation::suggested_days(30, core.start, core.end)
                    );
                }
            }
        }
    }
    #[test]
    fn invalid_exchange_preserves_totals_and_clamps_only_allowance_preview() {
        for previous in [None, Some(1_092_000)] {
            for costs in [None, Some(158_170)] {
                let mut entries = [entry(1, 1, 93_000), entry(u64::MAX, 2, 372_000)];
                entries[1].salary_exchange = SwedishTaxSalaryExchange {
                    is_some: 1,
                    sacrificed_salary: u32::MAX,
                    employer_adds_uplift: 1,
                    uplift_basis_points: 576,
                    previous_year_pension_salary_basis: previous.into(),
                    pension_and_insurance_costs_before_exchange: costs.into(),
                };
                let request = request(&entries);
                let (_, _, core) = plan_from_request(request).unwrap();
                let result = unsafe { swedish_tax_plan_support(&request) };
                assert_eq!(result.status, STATUS_OK);
                assert_eq!(result.issue_kind, 2);
                assert_eq!(result.issue_entry_id, u64::MAX);
                assert_eq!(result.entries_count, 2);
                let row = unsafe { *result.entries.add(1) };
                let expected = core.salary_exchange_allowance(u64::MAX).unwrap();
                assert_eq!(row.allowance.maximum_sacrifice, expected.maximum_sacrifice);
                assert_eq!(result.issue_maximum, expected.maximum_sacrifice);
                assert_eq!(
                    row.allowance.pension_salary_basis_after,
                    expected.pension_salary_basis_after
                );
                assert_eq!(row.allowance.ceiling, expected.ceiling);
                assert_eq!(
                    row.allowance.selected_exchange_contribution,
                    expected.selected_exchange_contribution
                );
                assert_eq!(row.salary_exchange_sacrifice, 372_000);
                assert_eq!(result.totals.work_income, core.totals().work_income);
                assert_eq!(entries[1].salary_exchange.sacrificed_salary, u32::MAX);
                unsafe { swedish_tax_plan_support_free(result) };
                let projection = unsafe { swedish_tax_calculate_plan(&request) };
                assert_eq!(projection.status, STATUS_INVALID_INPUT);
                unsafe { swedish_tax_calculation_result_free(projection) };
            }
        }
    }
    #[test]
    fn validation_precedence_empty_and_invalid_requests_and_ownership() {
        let mut entries = [entry(123, 1, 55_000), entry(456, 2, 500_000)];
        entries[0].start.month = 12;
        entries[0].end.month = 1;
        entries[1].salary_exchange.is_some = 1;
        entries[1].salary_exchange.sacrificed_salary = u32::MAX;
        for _ in 0..1000 {
            let r = unsafe { swedish_tax_plan_support(&request(&entries)) };
            assert_eq!(r.status, STATUS_OK);
            assert_eq!(r.issue_kind, 1);
            assert_eq!(r.issue_entry_id, 123);
            assert_eq!(r.totals.work_income, 0);
            unsafe { swedish_tax_plan_support_free(r) };
        }
        let r = unsafe { swedish_tax_plan_support(&request(&[])) };
        assert_eq!(r.status, STATUS_OK);
        assert_eq!(r.entries_count, 0);
        assert!(r.entries.is_null());
        unsafe { swedish_tax_plan_support_free(r) };
        let r = unsafe { swedish_tax_plan_support(std::ptr::null()) };
        assert_eq!(r.status, STATUS_INVALID_INPUT);
        unsafe { swedish_tax_plan_support_free(r) };
        assert_eq!(
            unsafe { swedish_tax_entry_support(std::ptr::null()) }.status,
            STATUS_INVALID_INPUT
        );
        entries[1].id = entries[0].id;
        let output = unsafe { swedish_tax_plan_support(&request(&entries)) };
        assert_eq!(output.status, STATUS_INVALID_INPUT);
        unsafe { swedish_tax_plan_support_free(output) };
    }
}
