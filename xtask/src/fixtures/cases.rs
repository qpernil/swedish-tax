use serde_json::{Value, json};
use swedish_tax::*;

pub fn cases() -> Vec<(&'static str, Value)> {
    let request = |plan: IncomePlan, table: u8, age_group: TaxAgeGroup| {
        let mut plan = serde_json::to_value(plan).unwrap();
        plan.as_object_mut().unwrap().remove("next_id");
        json!({"table":table,"age_group":age_group,"plan":plan})
    };
    let simple = request(
        IncomePlan::with_monthly_salary(DEFAULT_MONTHLY_INCOME),
        32,
        TaxAgeGroup::Under66AtYearStart,
    );
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    plan.set_adjustment_enabled(true);
    plan.adjustment_percent = Some(28);
    let salary = &mut plan.entries[0];
    salary.description = "Own company salary".into();
    salary.end = Date2026::new(10, 18);
    salary.use_annual_daily_rate_for_partial_months = true;
    salary.own_company_sourced = true;
    salary.use_full_year_projection_as_adjustment_basis = true;
    salary.additional_withholding_per_payment = Some(1500);
    salary.regular_pension_premium = Some(RegularPensionPremium {
        monthly_override: Some(15000),
    });
    let mut vacation = VacationCompensation::suggested(30, salary.start, salary.end);
    vacation.rate_basis_points = 560;
    vacation.pension_premium_override = Some(36000);
    salary.vacation_compensation = Some(vacation);
    let id = plan.add_entry(IncomeKind::OneTimeSalary);
    let exchange = plan.entries.iter_mut().find(|e| e.id == id).unwrap();
    exchange.description = "Termination payment".into();
    exchange.amount = 500000;
    exchange.own_company_sourced = true;
    exchange.included_in_pension_salary_basis = true;
    exchange.salary_exchange = Some(SalaryExchange {
        sacrificed_salary: 100000,
        employer_adds_uplift: true,
        uplift_basis_points: 576,
        previous_year_pension_salary_basis: Some(1116000),
        pension_and_insurance_costs_before_exchange: Some(210000),
    });
    plan.add_entry(IncomeKind::MonthlyOccupationalPension);
    let pension = plan.entries.last_mut().unwrap();
    pension.amount = 12000;
    pension.start = Date2026::new(11, 15);
    pension.payer_role = PayerRole::Secondary;
    pension.adjustment_applies = false;
    pension.actual_withholding = Some(5000);
    plan.add_entry(IncomeKind::OwnCompanyDividend);
    plan.entries.last_mut().unwrap().amount = 200000;
    plan.dividend_allowance = DividendAllowanceInputs2027 {
        one_person_company: false,
        ownership_basis_points: 6000,
        other_qualified_ownership_basis_points: 9000,
        spouse_ownership_basis_points: 4000,
        company_cash_payroll_2026: 3000000,
        highest_related_cash_salary_2026: 1000000,
        acquisition_cost: 200000,
        acquisition_cost_interest_basis_points: Some(1155),
        saved_allowance: 50000,
    };
    let full = request(plan.clone(), 34, TaxAgeGroup::AtLeast66AtYearStart);
    let mut cases = vec![("simple", simple.clone()), ("complete", full.clone())];
    plan.entries[1]
        .salary_exchange
        .as_mut()
        .unwrap()
        .sacrificed_salary = 500000;
    cases.push((
        "invalid_exchange",
        request(plan, 32, TaxAgeGroup::Under66AtYearStart),
    ));
    let mut invalid = simple.clone();
    invalid["plan"]["entries"][0]["start"] = json!({"month":12,"day":31});
    invalid["plan"]["entries"][0]["end"] = json!({"month":1,"day":1});
    cases.push(("invalid_period", invalid));
    let mut v = simple.clone();
    v["table"] = json!(28);
    cases.push(("unsupported_table", v));
    let mut v = simple.clone();
    v["plan"]["entries"][0]["unexpected"] = json!(true);
    cases.push(("unknown_field", v));
    let mut v = simple.clone();
    v["plan"]["entries"][0]["id"] = json!(18446744073709551614_u64);
    cases.push(("large_id", v));
    let mut v = simple.clone();
    v["plan"]["entries"][0]["amount"] = json!(0);
    cases.push(("zero", v));
    for (name, key, value) in [
        ("ownership_issue", "ownership_basis_points", json!(10001)),
        ("spouse_issue", "spouse_ownership_basis_points", json!(1)),
        ("payroll_issue", "one_person_company", json!(false)),
        ("interest_issue", "acquisition_cost", json!(200000)),
    ] {
        let mut v = simple.clone();
        v["plan"]["entries"][0]["own_company_sourced"] = json!(true);
        v["plan"]["dividend_allowance"][key] = value;
        cases.push((name, v));
    }
    let mut annual = IncomePlan::with_annual_salary(480000);
    annual.entries[0].regular_pension_premium = None;
    annual.add_entry(IncomeKind::AnnualOccupationalPension);
    annual.entries[1].amount = 120000;
    annual.entries[1].payer_role = PayerRole::Secondary;
    annual.entries[1].additional_withholding_per_payment = Some(1000);
    cases.push((
        "annual_mixed",
        request(annual, 42, TaxAgeGroup::Under66AtYearStart),
    ));
    let mut calendar = IncomePlan::with_monthly_salary(93000);
    calendar.entries[0].start = Date2026::new(2, 15);
    calendar.entries[0].end = Date2026::new(10, 18);
    calendar.entries[0].vacation_compensation = Some(VacationCompensation::suggested(
        30,
        calendar.entries[0].start,
        calendar.entries[0].end,
    ));
    calendar.add_entry(IncomeKind::OneTimeSalary);
    calendar.entries[1].amount = 200000;
    calendar.entries[1].included_in_pension_salary_basis = true;
    calendar.entries[1].salary_exchange = Some(SalaryExchange {
        sacrificed_salary: 50000,
        employer_adds_uplift: false,
        ..SalaryExchange::new()
    });
    cases.push((
        "calendar_current_basis",
        request(calendar, 29, TaxAgeGroup::Under66AtYearStart),
    ));
    let mut excessive = IncomePlan::with_monthly_salary(93_000);
    excessive.add_entry(IncomeKind::OneTimeSalary);
    excessive.entries[1].amount = 372_000;
    excessive.entries[1].included_in_pension_salary_basis = true;
    excessive.entries[1].salary_exchange = Some(SalaryExchange {
        sacrificed_salary: u32::MAX,
        ..SalaryExchange::new()
    });
    cases.push((
        "exchange_over_payment",
        request(excessive.clone(), 32, TaxAgeGroup::Under66AtYearStart),
    ));
    excessive.entries[1]
        .salary_exchange
        .as_mut()
        .unwrap()
        .previous_year_pension_salary_basis = Some(1_092_000);
    excessive.entries[1]
        .salary_exchange
        .as_mut()
        .unwrap()
        .pension_and_insurance_costs_before_exchange = Some(158_170);
    cases.push((
        "exchange_confirmed_costs",
        request(excessive.clone(), 32, TaxAgeGroup::AtLeast66AtYearStart),
    ));
    excessive.entries[0].amount = u32::MAX;
    excessive.entries[1].amount = u32::MAX;
    excessive.entries[1]
        .salary_exchange
        .as_mut()
        .unwrap()
        .previous_year_pension_salary_basis = None;
    excessive.entries[1]
        .salary_exchange
        .as_mut()
        .unwrap()
        .pension_and_insurance_costs_before_exchange = None;
    cases.push((
        "saturated_exchange",
        request(excessive, 32, TaxAgeGroup::Under66AtYearStart),
    ));
    cases
}
