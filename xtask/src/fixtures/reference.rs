//! Native reference serialization for shared test fixtures only.
//! This module is private to xtask; production clients use the C ABI.

use serde::Deserialize;
use serde_json::{Value, json};
use swedish_tax::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Date2026Input {
    month: u8,
    day: u8,
}
impl From<Date2026Input> for Date2026 {
    fn from(v: Date2026Input) -> Self {
        Self {
            month: v.month,
            day: v.day,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegularPensionPremiumInput {
    monthly_override: Option<u32>,
}
impl From<RegularPensionPremiumInput> for RegularPensionPremium {
    fn from(v: RegularPensionPremiumInput) -> Self {
        Self {
            monthly_override: v.monthly_override,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SalaryExchangeInput {
    sacrificed_salary: u32,
    employer_adds_uplift: bool,
    uplift_basis_points: u32,
    previous_year_pension_salary_basis: Option<u32>,
    pension_and_insurance_costs_before_exchange: Option<u32>,
}
impl From<SalaryExchangeInput> for SalaryExchange {
    fn from(v: SalaryExchangeInput) -> Self {
        Self {
            sacrificed_salary: v.sacrificed_salary,
            employer_adds_uplift: v.employer_adds_uplift,
            uplift_basis_points: v.uplift_basis_points,
            previous_year_pension_salary_basis: v.previous_year_pension_salary_basis,
            pension_and_insurance_costs_before_exchange: v
                .pension_and_insurance_costs_before_exchange,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VacationCompensationInput {
    annual_entitlement_days: u32,
    payout_days: u32,
    rate_basis_points: u32,
    included_in_pension_salary_basis: bool,
    pension_premium_override: Option<u32>,
}
impl From<VacationCompensationInput> for VacationCompensation {
    fn from(v: VacationCompensationInput) -> Self {
        Self {
            annual_entitlement_days: v.annual_entitlement_days,
            payout_days: v.payout_days,
            rate_basis_points: v.rate_basis_points,
            included_in_pension_salary_basis: v.included_in_pension_salary_basis,
            pension_premium_override: v.pension_premium_override,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DividendAllowanceInputs2027Input {
    one_person_company: bool,
    ownership_basis_points: u32,
    other_qualified_ownership_basis_points: u32,
    spouse_ownership_basis_points: u32,
    company_cash_payroll_2026: u32,
    highest_related_cash_salary_2026: u32,
    acquisition_cost: u32,
    acquisition_cost_interest_basis_points: Option<u32>,
    saved_allowance: u32,
}
impl From<DividendAllowanceInputs2027Input> for DividendAllowanceInputs2027 {
    fn from(v: DividendAllowanceInputs2027Input) -> Self {
        Self {
            one_person_company: v.one_person_company,
            ownership_basis_points: v.ownership_basis_points,
            other_qualified_ownership_basis_points: v.other_qualified_ownership_basis_points,
            spouse_ownership_basis_points: v.spouse_ownership_basis_points,
            company_cash_payroll_2026: v.company_cash_payroll_2026,
            highest_related_cash_salary_2026: v.highest_related_cash_salary_2026,
            acquisition_cost: v.acquisition_cost,
            acquisition_cost_interest_basis_points: v.acquisition_cost_interest_basis_points,
            saved_allowance: v.saved_allowance,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IncomeEntryInput {
    id: u64,
    description: String,
    kind: IncomeKind,
    amount: u32,
    start: Date2026Input,
    end: Date2026Input,
    use_annual_daily_rate_for_partial_months: bool,
    payer_role: PayerRole,
    own_company_sourced: bool,
    adjustment_applies: bool,
    use_full_year_projection_as_adjustment_basis: bool,
    additional_withholding_per_payment: Option<u32>,
    actual_withholding: Option<u32>,
    vacation_compensation: Option<VacationCompensationInput>,
    regular_pension_premium: Option<RegularPensionPremiumInput>,
    salary_exchange: Option<SalaryExchangeInput>,
    included_in_pension_salary_basis: bool,
}
impl From<IncomeEntryInput> for IncomeEntry {
    fn from(v: IncomeEntryInput) -> Self {
        Self {
            id: v.id,
            description: v.description,
            kind: v.kind,
            amount: v.amount,
            start: v.start.into(),
            end: v.end.into(),
            use_annual_daily_rate_for_partial_months: v.use_annual_daily_rate_for_partial_months,
            payer_role: v.payer_role,
            own_company_sourced: v.own_company_sourced,
            adjustment_applies: v.adjustment_applies,
            use_full_year_projection_as_adjustment_basis: v
                .use_full_year_projection_as_adjustment_basis,
            additional_withholding_per_payment: v.additional_withholding_per_payment,
            actual_withholding: v.actual_withholding,
            vacation_compensation: v.vacation_compensation.map(Into::into),
            regular_pension_premium: v.regular_pension_premium.map(Into::into),
            salary_exchange: v.salary_exchange.map(Into::into),
            included_in_pension_salary_basis: v.included_in_pension_salary_basis,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanInput {
    entries: Vec<IncomeEntryInput>,
    adjustment_percent: Option<u32>,
    dividend_allowance: DividendAllowanceInputs2027Input,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanRequest {
    table: u8,
    age_group: TaxAgeGroup,
    plan: PlanInput,
}

fn failure(kind: &str) -> Value {
    json!({"issue": {"kind": kind, "entry_id": null, "maximum": null}, "result": null})
}

pub(super) fn response(raw: Value) -> Value {
    let Ok(request) = serde_json::from_value::<PlanRequest>(raw) else {
        return failure("InvalidRequest");
    };
    if !(29..=42).contains(&request.table) {
        return failure("UnsupportedTaxTable");
    }
    if request.plan.entries.is_empty()
        || request.plan.entries.len() > 1000
        || request.plan.adjustment_percent.is_some_and(|p| p > 100)
    {
        return failure("InvalidRequest");
    }
    let mut ids = std::collections::HashSet::new();
    if request.plan.entries.iter().any(|e| {
        e.id == 0
            || !ids.insert(e.id)
            || !(1..=12).contains(&e.start.month)
            || !(1..=Date2026::days_in_month(e.start.month)).contains(&e.start.day)
            || !(1..=12).contains(&e.end.month)
            || !(1..=Date2026::days_in_month(e.end.month)).contains(&e.end.day)
    }) {
        return failure("InvalidRequest");
    }
    let mut plan = IncomePlan::with_annual_salary(0);
    plan.entries = request.plan.entries.into_iter().map(Into::into).collect();
    plan.adjustment_percent = request.plan.adjustment_percent;
    plan.dividend_allowance = request.plan.dividend_allowance.into();
    let issue = plan.validation_issue().map(|issue| match issue {
        IncomePlanValidationIssue::InvalidPaymentPeriod { entry_id } => {
            json!({"kind":"InvalidPaymentPeriod", "entry_id":entry_id,"maximum":null})
        }
        IncomePlanValidationIssue::SalaryExchangeExceedsAllowance { entry_id, maximum } => {
            json!({"kind":"SalaryExchangeExceedsAllowance", "entry_id":entry_id,"maximum":maximum})
        }
    });
    let totals = totals_json(plan.totals());
    let calculation = Calculation::new(request.table, request.age_group, &plan)
        .map(|v| calculation_json(v, totals.clone()));
    let withholding = plan.estimated_withholding(request.table, request.age_group);
    let entries: Vec<_> = plan.entries.iter().map(|e| entry_json(e, &plan)).collect();
    let dividend = match plan.dividend_allowance_2027() {
        Ok(v) => json!({"allowance":dividend_json(v),"issue":null}),
        Err(issue) => json!({"allowance":null,"issue":match issue {
            DividendAllowanceIssue::OwnershipExceedsOneHundredPercent => "OwnershipExceedsOneHundredPercent",
            DividendAllowanceIssue::SpouseOwnershipExceedsCompany => "SpouseOwnershipExceedsCompany",
            DividendAllowanceIssue::PersonalSalaryExceedsCompanyPayroll => "PersonalSalaryExceedsCompanyPayroll",
            DividendAllowanceIssue::MissingAcquisitionCostInterestRate => "MissingAcquisitionCostInterestRate",
        }}),
    };
    json!({"issue":issue,"result":{
        "calculation":calculation,"totals":totals,"entries":entries,
        "has_uniform_monthly_table_reference":plan.has_uniform_monthly_table_reference(),
        "salary_column":request.age_group.salary_column() as u8,
        "pension_column":request.age_group.pension_column() as u8,
        "withholding":{"total":withholding.total,"entries":withholding.entries.into_iter().map(withholding_json).collect::<Vec<_>>()},
        "dividend":dividend
    }})
}

fn annual_json(v: AnnualTax) -> Value {
    json!({
        "assessed_income": v.assessed_income,
        "basic_allowance": v.basic_allowance,
        "taxable_income": v.taxable_income,
        "state_income_tax": v.state_income_tax,
        "municipal_income_tax": v.municipal_income_tax,
        "burial_and_religious_fee": v.burial_and_religious_fee,
        "pension_fee": v.pension_fee,
        "pension_fee_credit": v.pension_fee_credit,
        "work_income_credit": v.work_income_credit,
        "sickness_compensation_credit": v.sickness_compensation_credit,
        "earned_income_credit": v.earned_income_credit,
        "public_service_fee": v.public_service_fee,
        "total": v.total,
        "additions_total": v.additions_total(),
        "credits_total": v.credits_total(),
    })
}

fn calibration_json(v: AdjustmentCalibration) -> Value {
    json!({
        "basis_income": v.basis_income,
        "percent": v.percent,
        "formula_tax_at_basis": v.formula_tax_at_basis,
        "assumed_tax_at_basis": v.assumed_tax_at_basis,
        "implied_tax_adjustment": v.implied_tax_adjustment,
        "projected_ordinary_tax": v.projected_ordinary_tax,
    })
}

fn balance_trace_json(v: AdjustmentBalanceTrace) -> Value {
    json!({
        "formula_tax_change": v.formula_tax_change,
        "withholding_change": v.withholding_change,
        "ordinary_balance": v.ordinary_balance,
    })
}

fn totals_json(v: IncomePlanTotals) -> Value {
    json!({
        "work_income": v.work_income,
        "pension_income": v.pension_income,
        "dividend_income": v.dividend_income,
        "sgi_annual_rate": v.sgi_annual_rate,
        "adjustment_basis_work_income": v.adjustment_basis_work_income,
        "pension_salary_basis": v.pension_salary_basis,
        "regular_pension_premiums": v.regular_pension_premiums,
        "vacation_pension_premiums": v.vacation_pension_premiums,
        "salary_exchange_sacrifice": v.salary_exchange_sacrifice,
        "salary_exchange_pension_contributions": v.salary_exchange_pension_contributions,
        "ordinary_income": v.ordinary_income(),
        "monthly_taxable_income": v.monthly_taxable_income(),
        "gross_income": v.gross_income(),
        "total_employer_pension_contributions": v.total_employer_pension_contributions(),
        "employer_pension_share_of_basis": v.employer_pension_share_of_basis(),
    })
}

fn allowance_json(v: SalaryExchangeAllowance) -> Value {
    json!({
        "ceiling": v.ceiling,
        "pension_salary_basis_before": v.pension_salary_basis_before,
        "pension_salary_basis_after": v.pension_salary_basis_after,
        "previous_year_pension_salary_basis": v.previous_year_pension_salary_basis,
        "pension_and_insurance_costs_before_exchange": v.pension_and_insurance_costs_before_exchange,
        "pension_contributions_before": v.pension_contributions_before,
        "regular_pension_premiums": v.regular_pension_premiums,
        "vacation_pension_premiums": v.vacation_pension_premiums,
        "other_exchange_contributions": v.other_exchange_contributions,
        "selected_exchange_contribution": v.selected_exchange_contribution,
        "total_employer_pension_contributions": v.total_employer_pension_contributions,
        "available_contribution": v.available_contribution,
        "maximum_sacrifice": v.maximum_sacrifice,
        "contribution_share_of_basis": v.contribution_share_of_basis(),
    })
}

fn dividend_json(v: DividendAllowance2027) -> Value {
    json!({
        "basic_amount": v.basic_amount,
        "owner_cash_salary": v.owner_cash_salary,
        "company_cash_payroll": v.company_cash_payroll,
        "joint_wage_basis": v.joint_wage_basis,
        "joint_wage_basis_after_deduction": v.joint_wage_basis_after_deduction,
        "wage_allowance_before_cap": v.wage_allowance_before_cap,
        "wage_cap_salary": v.wage_cap_salary,
        "wage_cap": v.wage_cap,
        "wage_allowance": v.wage_allowance,
        "acquisition_cost_interest_basis": v.acquisition_cost_interest_basis,
        "acquisition_cost_interest": v.acquisition_cost_interest,
        "saved_allowance": v.saved_allowance,
        "total": v.total,
        "tax_at_twenty_percent": v.tax_at_twenty_percent(),
        "net_after_twenty_percent_tax": v.net_after_twenty_percent_tax(),
    })
}

fn calculation_json(v: Calculation, totals: Value) -> Value {
    json!({
        "totals":totals,
        "monthly_income":v.monthly_income,
        "annual_income":v.annual_income,
        "ordinary_income":v.ordinary_income,
        "work_income":v.work_income,
        "pension_income":v.pension_income,
        "dividend_income":v.dividend_income,
        "sgi_annual_rate":v.sgi_annual_rate,
        "table_deduction":deduction_json(v.table_deduction),
        "annual_tax":annual_json(v.annual_tax),
        "adjustment_calibration":v.adjustment_calibration.map(calibration_json),
        "ordinary_final_tax":v.ordinary_final_tax,
        "dividend_tax":v.dividend_tax,
        "total_tax":v.total_tax,
        "withheld_tax":v.withheld_tax,
        "regular_pension_premiums":v.regular_pension_premiums,
        "vacation_pension_premiums":v.vacation_pension_premiums,
        "salary_exchange_sacrifice":v.salary_exchange_sacrifice,
        "salary_exchange_pension_contributions":v.salary_exchange_pension_contributions,
        "pension_salary_basis":v.pension_salary_basis,
        "employer_pension_contributions":v.employer_pension_contributions,
        "marginal_rate":v.marginal_rate,
        "pension_progress":basis_json(v.pension_progress),
        "sgi_progress":basis_json(v.sgi_progress),
        "table_reference_tax":v.table_reference_tax(),
        "table_reference_net":v.table_reference_net(),
        "annualized_table_reference_tax":v.annualized_table_reference_tax(),
        "effective_rate":v.effective_rate(),
        "employer_pension_share_of_basis":v.employer_pension_share_of_basis(),
        "annual_net":v.annual_net(),
        "cash_after_withholding":v.cash_after_withholding(),
        "tax_balance":v.tax_balance(),
        "adjustment_balance_trace":v.adjustment_balance_trace().map(balance_trace_json)
    })
}

fn deduction_json(v: TaxDeduction) -> Value {
    match v {
        TaxDeduction::Amount(value) => json!({"kind":"Amount","value":value}),
        TaxDeduction::Percent(value) => json!({"kind":"Percent","value":value}),
    }
}
fn basis_json(v: IncomeBasisEstimate) -> Value {
    match v {
        IncomeBasisEstimate::Estimated(p) => {
            json!({"kind":"Estimated","progress":{"estimated_basis":p.estimated_basis,"maximum_basis":p.maximum_basis,"percent_of_maximum":p.percent_of_maximum()}})
        }
        IncomeBasisEstimate::NotBasedOnSelectedIncome => {
            json!({"kind":"NotBasedOnSelectedIncome","progress":null})
        }
        IncomeBasisEstimate::RequiresAdditionalInformation => {
            json!({"kind":"RequiresAdditionalInformation","progress":null})
        }
    }
}
fn withholding_json(v: EntryWithholding) -> Value {
    let (kind, column, percent) = match v.rule {
        AppliedWithholding::ActualAmount => ("ActualAmount", None, None),
        AppliedWithholding::Table(c) => ("Table", Some(c as u8), None),
        AppliedWithholding::TableAndOneTime(c, p) => ("TableAndOneTime", Some(c as u8), Some(p)),
        AppliedWithholding::OneTimeTable(p) => ("OneTimeTable", None, Some(p)),
        AppliedWithholding::Secondary30 => {
            ("Secondary30", None, Some(SECONDARY_WITHHOLDING_PERCENT))
        }
        AppliedWithholding::AdjustmentPercent(p) => ("AdjustmentPercent", None, Some(p)),
        AppliedWithholding::None => ("None", None, None),
    };
    json!({"entry_id":v.entry_id,"gross":v.gross,"withheld":v.withheld,"regular_withheld":v.regular_withheld,
        "supplemental_withheld":v.supplemental_withheld,"additional_withheld":v.additional_withheld,
        "rule":{"kind":kind,"column":column,"percent":percent}})
}
fn entry_json(e: &IncomeEntry, plan: &IncomePlan) -> Value {
    json!({
        "entry_id":e.id,
        "annual_amount":e.annual_amount(),
        "total_annual_amount":e.total_annual_amount(),
        "is_valid":e.is_valid(),
        "withholding_payment_count":e.withholding_payment_count(),
        "requested_additional_withholding":e.requested_additional_withholding(),
        "vacation_compensation_amount":e.vacation_compensation_amount(),
        "regular_pension_premium_amount":e.regular_pension_premium_amount(),
        "vacation_pension_premium_amount":e.vacation_pension_premium_amount(),
        "pension_salary_basis_amount":e.pension_salary_basis_amount(),
        "salary_exchange_sacrifice":e.salary_exchange_sacrifice(),
        "salary_exchange_pension_contribution":e.salary_exchange_pension_contribution(),
        "pension_benchmark_monthly":e.regular_pension_benchmark_monthly().unwrap_or(0),
        "suggested_vacation_days":VacationCompensation::suggested_days(e.vacation_compensation.map_or(25, |v| v.annual_entitlement_days),e.start,e.end),
        "vacation_amount_per_day":e.vacation_compensation.map_or(0.0, |v| v.amount_per_day(e.amount)),
        "allowance":plan.salary_exchange_allowance(e.id).map(allowance_json),
        "monthly_amounts":(1..=12).map(|m| e.amount_for_month(m)).collect::<Vec<_>>()
    })
}
