use super::*;
use crate::{Calculation, TaxAgeGroup, TaxColumn};

#[test]
fn exact_date_monthly_income_prorates_partial_months() {
    let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
    entry.amount = 93_000;
    entry.start = Date2026::new(1, 1);
    entry.end = Date2026::new(10, 18);

    assert_eq!(entry.amount_for_month(9), 93_000);
    assert_eq!(entry.amount_for_month(10), 54_000);
    assert_eq!(entry.amount_for_month(11), 0);
    assert_eq!(entry.annual_amount(), 891_000);
}

#[test]
fn annual_daily_rate_is_optional_and_keeps_full_months_unchanged() {
    let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
    entry.amount = 93_000;
    entry.start = Date2026::new(1, 1);
    entry.end = Date2026::new(10, 18);
    entry.use_annual_daily_rate_for_partial_months = true;

    assert_eq!(entry.amount_for_month(9), 93_000);
    assert_eq!(entry.amount_for_month(10), 55_036);
    assert_eq!(entry.annual_amount(), 892_036);
}

#[test]
fn adjustment_basis_projects_recurring_salary_over_the_full_year() {
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    plan.entries[0].end = Date2026::new(10, 18);
    plan.entries[0].use_full_year_projection_as_adjustment_basis = true;

    let totals = plan.totals();
    assert_eq!(totals.work_income, 891_000);
    assert_eq!(totals.adjustment_basis_work_income, 1_116_000);
}

#[test]
fn regular_pension_benchmark_matches_the_confirmed_salary_and_period() {
    assert_eq!(RegularPensionPremium::benchmark_monthly(93_000), 14_608);

    let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
    entry.amount = 93_000;
    entry.end = Date2026::new(10, 18);
    assert_eq!(entry.regular_pension_premium_amount(), 139_954);

    entry.regular_pension_premium = Some(RegularPensionPremium {
        monthly_override: Some(14_600),
    });
    assert_eq!(entry.regular_pension_premium_amount(), 139_877);
}

#[test]
fn salary_exchange_uses_remaining_pension_allowance_and_uplift() {
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    plan.entries[0].end = Date2026::new(10, 18);
    plan.entries[0].vacation_compensation = Some(VacationCompensation::suggested(
        30,
        plan.entries[0].start,
        plan.entries[0].end,
    ));

    let lump_id = plan.add_entry(IncomeKind::OneTimeSalary);
    let lump = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap();
    lump.amount = 372_000;
    lump.salary_exchange = Some(SalaryExchange::new());

    let allowance = plan.salary_exchange_allowance(lump_id).unwrap();
    assert_eq!(allowance.pension_salary_basis_before, 1_011_528);
    assert_eq!(allowance.pension_salary_basis_after, 1_011_528);
    assert_eq!(allowance.ceiling, 354_034);
    assert_eq!(allowance.regular_pension_premiums, 139_954);
    assert_eq!(allowance.vacation_pension_premiums, 36_159);
    assert_eq!(allowance.available_contribution, 177_921);
    assert_eq!(allowance.maximum_sacrifice, 168_231);

    plan.entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap()
        .salary_exchange
        .as_mut()
        .unwrap()
        .sacrificed_salary = 100_000;
    let partial_totals = plan.totals();
    assert_eq!(partial_totals.salary_exchange_sacrifice, 100_000);
    assert_eq!(
        partial_totals.salary_exchange_pension_contributions,
        105_760
    );
    assert_eq!(partial_totals.work_income, 1_283_528);

    plan.entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap()
        .salary_exchange
        .as_mut()
        .unwrap()
        .sacrificed_salary = allowance.maximum_sacrifice;

    let totals = plan.totals();
    assert_eq!(totals.work_income, 1_215_297);
    assert_eq!(totals.salary_exchange_sacrifice, 168_231);
    assert_eq!(totals.salary_exchange_pension_contributions, 177_921);
    assert_eq!(totals.total_employer_pension_contributions(), 354_034);
    assert_eq!(
        totals.employer_pension_share_of_basis(),
        f64::from(354_034) * 100.0 / f64::from(1_011_528)
    );
    let applied_allowance = plan.salary_exchange_allowance(lump_id).unwrap();
    assert_eq!(
        applied_allowance.total_employer_pension_contributions,
        354_034
    );
    assert_eq!(
        applied_allowance.contribution_share_of_basis(),
        f64::from(354_034) * 100.0 / f64::from(1_011_528)
    );
}

#[test]
fn pension_allowance_ceiling_never_rounds_above_thirty_five_percent() {
    let basis = 1_011_528;
    let ceiling = SalaryExchange::allowance_ceiling(basis);

    assert_eq!(ceiling, 354_034);
    assert!(u64::from(ceiling) * 10_000 <= u64::from(basis) * 3_500);
    assert!(u64::from(ceiling + 1) * 10_000 > u64::from(basis) * 3_500);
}

#[test]
fn previous_year_pension_salary_keeps_the_ceiling_fixed_like_yubico() {
    let context = SalaryExchangeContext {
        regular_pension_premiums: 158_170,
        vacation_pension_premiums: 0,
        other_exchange_contributions: 0,
        other_pension_salary_basis: 0,
    };
    let mut exchange = SalaryExchange::new();
    exchange.sacrificed_salary = 211_000;
    exchange.uplift_basis_points = 580;
    exchange.previous_year_pension_salary_basis = Some(1_092_000);
    exchange.pension_and_insurance_costs_before_exchange = Some(158_170);

    let allowance = context.allowance_for(372_000, false, exchange);

    assert_eq!(
        allowance.previous_year_pension_salary_basis,
        Some(1_092_000)
    );
    assert_eq!(allowance.pension_salary_basis_before, 1_092_000);
    assert_eq!(allowance.pension_salary_basis_after, 1_092_000);
    assert_eq!(allowance.pension_contributions_before, 158_170);
    assert_eq!(allowance.ceiling, 382_200);
    assert_eq!(allowance.available_contribution, 224_030);
    assert_eq!(allowance.maximum_sacrifice, 211_749);
    assert_eq!(allowance.selected_exchange_contribution, 223_238);
    assert_eq!(allowance.total_employer_pension_contributions, 381_408);
}

#[test]
fn increased_pension_contributions_invalidate_an_excessive_salary_exchange() {
    let mut plan = IncomePlan::with_annual_salary(1_000_000);
    let lump_id = plan.add_entry(IncomeKind::OneTimeSalary);
    let lump = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap();
    lump.amount = 300_000;
    lump.salary_exchange = Some(SalaryExchange::new());

    let original_maximum = plan
        .salary_exchange_allowance(lump_id)
        .unwrap()
        .maximum_sacrifice;
    assert!(original_maximum > 0);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap()
        .salary_exchange
        .as_mut()
        .unwrap()
        .sacrificed_salary = original_maximum;
    assert_eq!(plan.validation_issue(), None);

    plan.entries[0]
        .regular_pension_premium
        .as_mut()
        .unwrap()
        .monthly_override = Some(25_000);
    let reduced_maximum = plan
        .salary_exchange_allowance(lump_id)
        .unwrap()
        .maximum_sacrifice;
    assert!(reduced_maximum < original_maximum);
    assert_eq!(
        plan.validation_issue(),
        Some(IncomePlanValidationIssue::SalaryExchangeExceedsAllowance {
            entry_id: lump_id,
            maximum: reduced_maximum,
        })
    );
    assert!(Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).is_none());
}

#[test]
fn complete_salary_vacation_lump_sum_and_maximum_exchange_scenario() {
    const MONTHLY_SALARY: u32 = 93_000;

    let mut plan = IncomePlan::with_monthly_salary(MONTHLY_SALARY);
    let salary = &mut plan.entries[0];
    salary.start = Date2026::new(1, 1);
    salary.end = Date2026::new(10, 18);
    salary.vacation_compensation = Some(VacationCompensation::suggested(
        30,
        salary.start,
        salary.end,
    ));

    assert_eq!(salary.annual_amount(), 891_000);
    assert_eq!(salary.regular_pension_premium_amount(), 139_954);
    assert_eq!(salary.vacation_compensation.unwrap().payout_days, 24);
    assert_eq!(salary.vacation_compensation_amount(), 120_528);
    assert_eq!(salary.vacation_pension_premium_amount(), 36_159);

    let lump_id = plan.add_entry(IncomeKind::OneTimeSalary);
    let lump = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap();
    lump.amount = MONTHLY_SALARY * 4;
    lump.salary_exchange = Some(SalaryExchange::new());

    let allowance = plan.salary_exchange_allowance(lump_id).unwrap();
    assert_eq!(allowance.pension_salary_basis_before, 1_011_528);
    assert_eq!(allowance.ceiling, 354_034);
    assert_eq!(allowance.available_contribution, 177_921);
    assert_eq!(allowance.maximum_sacrifice, 168_231);

    let lump = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == lump_id)
        .unwrap();
    lump.salary_exchange.as_mut().unwrap().sacrificed_salary = allowance.maximum_sacrifice;
    assert_eq!(lump.total_annual_amount(), 203_769);
    assert_eq!(lump.salary_exchange_pension_contribution(), 177_921);

    let totals = plan.totals();
    assert_eq!(totals.work_income, 1_215_297);
    assert_eq!(totals.pension_salary_basis, 1_011_528);
    assert_eq!(totals.regular_pension_premiums, 139_954);
    assert_eq!(totals.vacation_pension_premiums, 36_159);
    assert_eq!(totals.salary_exchange_sacrifice, 168_231);
    assert_eq!(totals.salary_exchange_pension_contributions, 177_921);
    assert_eq!(totals.total_employer_pension_contributions(), 354_034);

    let mut one_krona_too_much = SalaryExchange::new();
    one_krona_too_much.sacrificed_salary = allowance.maximum_sacrifice + 1;
    assert!(
        totals
            .regular_pension_premiums
            .saturating_add(totals.vacation_pension_premiums)
            .saturating_add(one_krona_too_much.pension_contribution())
            > allowance.ceiling
    );
}

#[test]
fn vacation_compensation_saturates_at_numeric_limits() {
    let vacation = VacationCompensation {
        annual_entitlement_days: u32::MAX,
        payout_days: u32::MAX,
        rate_basis_points: u32::MAX,
        included_in_pension_salary_basis: true,
        pension_premium_override: None,
    };

    assert_eq!(vacation.amount(u32::MAX), u32::MAX);
}

#[test]
fn same_year_vacation_compensation_is_suggested_but_days_remain_editable() {
    let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
    entry.amount = 93_000;
    entry.start = Date2026::new(1, 1);
    entry.end = Date2026::new(10, 18);
    entry.vacation_compensation = Some(VacationCompensation::suggested(30, entry.start, entry.end));

    assert_eq!(entry.vacation_compensation.unwrap().payout_days, 24);
    assert_eq!(entry.vacation_compensation.unwrap().rate_basis_points, 540);
    assert_eq!(entry.vacation_compensation_amount(), 120_528);
    assert_eq!(entry.vacation_pension_premium_amount(), 36_159);
    assert_eq!(entry.pension_salary_basis_amount(), 1_011_528);
    assert_eq!(entry.total_annual_amount(), 1_011_528);

    entry.vacation_compensation.as_mut().unwrap().payout_days = 20;
    assert_eq!(entry.vacation_compensation_amount(), 100_440);

    entry
        .vacation_compensation
        .as_mut()
        .unwrap()
        .rate_basis_points = 500;
    assert_eq!(entry.vacation_compensation_amount(), 93_000);
}

#[test]
fn pensionability_and_actual_vacation_premium_are_editable() {
    let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
    entry.amount = 93_000;
    entry.end = Date2026::new(10, 18);
    entry.vacation_compensation = Some(VacationCompensation::suggested(30, entry.start, entry.end));

    let vacation = entry.vacation_compensation.as_mut().unwrap();
    vacation.pension_premium_override = Some(30_000);
    assert_eq!(entry.vacation_pension_premium_amount(), 30_000);

    entry
        .vacation_compensation
        .as_mut()
        .unwrap()
        .included_in_pension_salary_basis = false;
    assert_eq!(entry.vacation_pension_premium_amount(), 0);
    assert_eq!(entry.pension_salary_basis_amount(), 891_000);
}

#[test]
fn vacation_compensation_adds_pgi_and_one_time_withholding_but_not_sgi() {
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    plan.entries[0].end = Date2026::new(10, 18);
    plan.entries[0].vacation_compensation = Some(VacationCompensation::suggested(
        30,
        plan.entries[0].start,
        plan.entries[0].end,
    ));

    let totals = plan.totals();
    assert_eq!(totals.work_income, 1_011_528);
    assert_eq!(totals.sgi_annual_rate, 1_116_000);

    let withholding = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    assert!(matches!(
        withholding.entries[0].rule,
        AppliedWithholding::TableAndOneTime(TaxColumn::Column1, 54)
    ));
}

#[test]
fn adding_an_income_preserves_every_field_in_existing_rows() {
    let mut plan = IncomePlan::with_annual_salary(930_000);
    plan.entries[0].description = "Existing salary".to_owned();
    plan.entries[0].payer_role = PayerRole::Secondary;
    plan.entries[0].adjustment_applies = true;
    plan.entries[0].additional_withholding_per_payment = Some(3_700);
    let existing = plan.entries[0].clone();

    plan.add_entry(IncomeKind::MonthlySalary);

    assert_eq!(plan.entries[0], existing);
    assert_eq!(plan.entries.len(), 2);
    assert_ne!(plan.entries[0].id, plan.entries[1].id);
}

#[test]
fn removing_every_income_kind_by_id_at_any_position_restores_the_calculation() {
    let mut baseline = IncomePlan::with_monthly_salary(45_000);
    baseline.entries[0].description = "Salary".to_owned();
    let pension_id = baseline.add_entry(IncomeKind::AnnualOccupationalPension);
    let pension = baseline
        .entries
        .iter_mut()
        .find(|entry| entry.id == pension_id)
        .unwrap();
    pension.description = "Pension".to_owned();
    pension.amount = 120_000;
    pension.payer_role = PayerRole::Secondary;
    let baseline_calculation =
        crate::Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &baseline).unwrap();

    for kind in IncomeKind::ALL {
        for target_index in 0..=baseline.entries.len() {
            let mut plan = baseline.clone();
            let target_id = plan.add_entry(kind);
            let appended_index = plan.entries.len() - 1;
            plan.entries[appended_index].description = format!("Temporary {kind:?}");
            plan.entries[appended_index].amount = 60_000;
            let added_entry = plan.entries.remove(appended_index);
            plan.entries.insert(target_index, added_entry);

            assert_eq!(plan.entries[target_index].id, target_id);
            assert_ne!(
                crate::Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan),
                Some(baseline_calculation),
                "{kind:?} at index {target_index} did not affect the calculation"
            );

            plan.remove_entry(target_id);

            assert_eq!(plan.entries, baseline.entries);
            assert_eq!(
                crate::Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan),
                Some(baseline_calculation),
                "{kind:?} at index {target_index} did not restore the calculation"
            );
        }
    }
}

#[test]
fn enabling_adjustment_defaults_to_non_dividend_main_payers() {
    let mut plan = IncomePlan::with_annual_salary(930_000);
    let secondary_id = plan.add_entry(IncomeKind::AnnualOccupationalPension);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == secondary_id)
        .unwrap()
        .payer_role = PayerRole::Secondary;
    let dividend_id = plan.add_entry(IncomeKind::OwnCompanyDividend);

    plan.set_adjustment_enabled(true);

    assert_eq!(plan.adjustment_percent, Some(30));
    assert!(plan.entries[0].adjustment_applies);
    assert!(
        !plan
            .entries
            .iter()
            .find(|entry| entry.id == secondary_id)
            .unwrap()
            .adjustment_applies
    );
    assert!(
        !plan
            .entries
            .iter()
            .find(|entry| entry.id == dividend_id)
            .unwrap()
            .adjustment_applies
    );
}

#[test]
fn new_main_payers_default_to_adjustment_when_a_decision_is_enabled() {
    let mut plan = IncomePlan::with_annual_salary(930_000);
    plan.set_adjustment_enabled(true);

    let pension_id = plan.add_entry(IncomeKind::AnnualOccupationalPension);
    let pension = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == pension_id)
        .unwrap();
    assert!(pension.adjustment_applies);

    pension.set_payer_role(PayerRole::Secondary, true);
    assert!(!pension.adjustment_applies);
    pension.set_payer_role(PayerRole::Main, true);
    assert!(pension.adjustment_applies);
}

#[test]
fn payer_can_override_the_main_payer_adjustment_default() {
    let mut plan = IncomePlan::with_annual_salary(930_000);
    plan.set_adjustment_enabled(true);
    plan.entries[0].adjustment_applies = false;

    plan.entries[0].set_payer_role(PayerRole::Main, true);

    assert!(!plan.entries[0].adjustment_applies);
}

#[test]
fn additional_withholding_editor_default_comes_from_the_income_entry() {
    let mut entry = IncomeEntry::new(1, IncomeKind::AnnualSalary);

    entry.set_additional_withholding_enabled(true);
    assert_eq!(entry.additional_withholding_per_payment, Some(1_000));

    entry.set_additional_withholding_enabled(false);
    assert_eq!(entry.additional_withholding_per_payment, None);
}

#[test]
fn actual_withholding_editor_default_comes_from_the_income_entry() {
    let mut entry = IncomeEntry::new(1, IncomeKind::AnnualSalary);

    entry.set_actual_withholding_enabled(true);
    assert_eq!(entry.actual_withholding, Some(0));

    entry.set_actual_withholding_enabled(false);
    assert_eq!(entry.actual_withholding, None);
}

#[test]
fn monthly_salary_plan_defaults_to_the_full_calendar_year() {
    let plan = IncomePlan::with_monthly_salary(55_033);
    let totals = plan.totals();

    assert_eq!(plan.entries[0].kind, IncomeKind::MonthlySalary);
    assert_eq!(plan.entries[0].start, Date2026::new(1, 1));
    assert_eq!(plan.entries[0].end, Date2026::new(12, 31));
    assert_eq!(plan.entries[0].annual_amount(), 660_396);
    assert_eq!(totals.monthly_taxable_income(), 55_033);
}

#[test]
fn scenario_totals_keep_salary_pension_and_dividend_separate() {
    let mut plan = IncomePlan::with_annual_salary(0);
    plan.entries.clear();

    let salary_id = plan.add_entry(IncomeKind::MonthlySalary);
    let salary = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == salary_id)
        .unwrap();
    salary.amount = 93_000;
    salary.end = Date2026::new(10, 18);

    let severance_id = plan.add_entry(IncomeKind::OneTimeSalary);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == severance_id)
        .unwrap()
        .amount = 372_000;
    let vacation_id = plan.add_entry(IncomeKind::OneTimeSalary);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == vacation_id)
        .unwrap()
        .amount = 120_528;

    let pension_id = plan.add_entry(IncomeKind::MonthlyOccupationalPension);
    let pension = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == pension_id)
        .unwrap();
    pension.amount = 27_500;
    pension.start = Date2026::new(8, 1);

    let dividend_id = plan.add_entry(IncomeKind::OwnCompanyDividend);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == dividend_id)
        .unwrap()
        .amount = 200_000;

    assert_eq!(
        plan.totals(),
        IncomePlanTotals {
            work_income: 1_383_528,
            pension_income: 137_500,
            dividend_income: 200_000,
            sgi_annual_rate: 1_116_000,
            adjustment_basis_work_income: 0,
            pension_salary_basis: 891_000,
            regular_pension_premiums: 139_954,
            vacation_pension_premiums: 0,
            salary_exchange_sacrifice: 0,
            salary_exchange_pension_contributions: 0,
        }
    );
}

#[test]
fn withholding_rules_and_additional_amount_compose() {
    let mut plan = IncomePlan::with_annual_salary(700_000);
    let secondary_id = plan.add_entry(IncomeKind::AnnualOccupationalPension);
    let secondary = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == secondary_id)
        .unwrap();
    secondary.amount = 100_000;
    secondary.payer_role = PayerRole::Secondary;

    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    let pension = summary
        .entries
        .iter()
        .find(|entry| entry.entry_id == secondary_id)
        .unwrap();
    assert_eq!(pension.withheld, 30_000);
    assert_eq!(pension.rule, AppliedWithholding::Secondary30);

    plan.adjustment_percent = Some(38);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == secondary_id)
        .unwrap()
        .adjustment_applies = true;
    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    let pension = summary
        .entries
        .iter()
        .find(|entry| entry.entry_id == secondary_id)
        .unwrap();
    assert_eq!(pension.withheld, 38_000);
    assert_eq!(pension.rule, AppliedWithholding::AdjustmentPercent(38));

    plan.entries
        .iter_mut()
        .find(|entry| entry.id == secondary_id)
        .unwrap()
        .additional_withholding_per_payment = Some(500);
    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    let pension = summary
        .entries
        .iter()
        .find(|entry| entry.entry_id == secondary_id)
        .unwrap();
    assert_eq!(pension.withheld, 44_000);
    assert_eq!(pension.additional_withheld, 6_000);
    assert_eq!(pension.rule, AppliedWithholding::AdjustmentPercent(38));
}

#[test]
fn actual_withholding_overrides_estimates_for_every_income_kind() {
    let mut plan = IncomePlan::with_annual_salary(0);
    plan.entries.clear();
    let mut expected_total = 0_u32;

    for (index, kind) in IncomeKind::ALL.into_iter().enumerate() {
        let id = plan.add_entry(kind);
        let entry = plan
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .unwrap();
        entry.amount = 100_000;
        entry.adjustment_applies = true;
        entry.additional_withholding_per_payment = Some(42);
        let actual = (index as u32 + 1) * 1_000;
        entry.actual_withholding = Some(actual);
        expected_total = expected_total.saturating_add(actual);
    }
    plan.adjustment_percent = Some(38);

    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);

    assert_eq!(summary.total, expected_total);
    assert!(summary.entries.iter().enumerate().all(|(index, entry)| {
        entry.rule == AppliedWithholding::ActualAmount
            && entry.withheld == (index as u32 + 1) * 1_000
            && entry.additional_withheld == 0
    }));
}

#[test]
fn additional_withholding_uses_payment_count_and_cannot_exceed_gross() {
    let mut plan = IncomePlan::with_monthly_salary(10_000);
    plan.entries[0].start = Date2026::new(3, 15);
    plan.entries[0].end = Date2026::new(5, 1);
    plan.entries[0].additional_withholding_per_payment = Some(1_000);

    let estimate = plan
        .estimated_withholding(32, TaxAgeGroup::Under66AtYearStart)
        .entries[0];
    assert_eq!(plan.entries[0].withholding_payment_count(), 3);
    assert_eq!(estimate.additional_withheld, 3_000);

    plan.entries[0].additional_withholding_per_payment = Some(u32::MAX);
    let estimate = plan
        .estimated_withholding(32, TaxAgeGroup::Under66AtYearStart)
        .entries[0];
    assert_eq!(estimate.withheld, estimate.gross);
    assert_eq!(
        estimate.additional_withheld,
        estimate.gross - estimate.regular_withheld
    );
}

#[test]
fn own_company_source_only_totals_cash_salary_types() {
    let mut plan = IncomePlan::with_monthly_salary(10_000);
    plan.entries[0].own_company_sourced = true;
    let one_time_id = plan.add_entry(IncomeKind::OneTimeSalary);
    let one_time = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == one_time_id)
        .unwrap();
    one_time.amount = 20_000;
    one_time.own_company_sourced = true;
    let pension_id = plan.add_entry(IncomeKind::AnnualOccupationalPension);
    let pension = plan
        .entries
        .iter_mut()
        .find(|entry| entry.id == pension_id)
        .unwrap();
    pension.amount = 50_000;
    pension.own_company_sourced = true;

    assert_eq!(plan.own_company_sourced_work_income(), 140_000);
}

#[test]
fn marked_2026_salary_feeds_the_preliminary_2027_dividend_allowance() {
    let mut plan = IncomePlan::with_monthly_salary(50_000);
    plan.entries[0].own_company_sourced = true;

    let allowance = plan.dividend_allowance_2027().unwrap();

    assert_eq!(allowance.owner_cash_salary, 600_000);
    assert_eq!(allowance.company_cash_payroll, 600_000);
    assert_eq!(allowance.total, 333_600);
}

#[test]
fn main_payer_one_time_salary_uses_the_engang_table() {
    let mut plan = IncomePlan::with_annual_salary(700_000);
    let id = plan.add_entry(IncomeKind::OneTimeSalary);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .unwrap()
        .amount = 100_000;

    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    let payment = summary
        .entries
        .iter()
        .find(|entry| entry.entry_id == id)
        .unwrap();
    assert_eq!(payment.withheld, 54_000);
    assert_eq!(payment.rule, AppliedWithholding::OneTimeTable(54));
}

#[test]
fn own_company_dividend_has_tax_liability_but_no_default_withholding() {
    let mut plan = IncomePlan::with_annual_salary(0);
    plan.entries[0].kind = IncomeKind::OwnCompanyDividend;
    plan.entries[0].amount = 200_000;

    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    assert_eq!(summary.total, 0);
    assert_eq!(summary.entries[0].rule, AppliedWithholding::None);

    plan.entries[0].actual_withholding = Some(25_000);
    let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
    assert_eq!(summary.total, 25_000);
    assert_eq!(summary.entries[0].rule, AppliedWithholding::ActualAmount);
}
