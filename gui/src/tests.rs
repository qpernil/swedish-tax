use super::*;

#[test]
fn table_32_is_selected_by_default() {
    assert_eq!(TaxApp::default().table, 32);
}

#[test]
fn newly_added_income_becomes_the_selected_row() {
    let mut app = TaxApp::default();
    let id = app.add_income_for_editing();

    assert_eq!(app.selected_income_entry, Some(id));
    assert_eq!(app.income_plan.entries.last().unwrap().id, id);
}

#[test]
fn persisted_state_restores_the_complete_plan_and_settings() {
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    plan.adjustment_percent = Some(33);
    let pension_id = plan.add_entry(IncomeKind::MonthlyOccupationalPension);
    plan.entries
        .iter_mut()
        .find(|entry| entry.id == pension_id)
        .unwrap()
        .amount = 27_500;
    let app = TaxApp::from_persisted_state(PersistedAppState::new(
        34,
        TaxAgeGroup::AtLeast66AtYearStart,
        plan.clone(),
    ));

    assert_eq!(app.table, 34);
    assert_eq!(app.age_group, TaxAgeGroup::AtLeast66AtYearStart);
    assert_eq!(app.income_plan, plan);
    assert_eq!(app.persisted_state().income_plan, plan);
}

#[test]
fn monthly_table_reference_requires_one_uniform_full_year_salary() {
    let mut plan = IncomePlan::with_monthly_salary(93_000);
    assert!(plan.has_uniform_monthly_table_reference());

    plan.entries[0].end = Date2026::new(10, 18);
    assert!(!plan.has_uniform_monthly_table_reference());

    plan = IncomePlan::with_annual_salary(1_116_000);
    assert!(plan.has_uniform_monthly_table_reference());

    plan.add_entry(IncomeKind::AnnualOccupationalPension);
    assert!(!plan.has_uniform_monthly_table_reference());
}

#[test]
fn formatting_groups_sek_without_locale_dependencies() {
    assert_eq!(format_sek(0), "0 SEK");
    assert_eq!(format_sek(1_234_567), "1 234 567 SEK");
    assert_eq!(
        tax_balance_summary(TaxBalance::Debt(2_400)),
        "Expected balance: −2 400 SEK · Tax debt"
    );
    assert_eq!(
        tax_balance_summary(TaxBalance::Refund(350)),
        "Expected balance: +350 SEK · Tax refund"
    );
}

#[test]
fn basis_point_percentages_use_exact_text_conversion() {
    assert_eq!(format_basis_points_percentage(0), "0");
    assert_eq!(format_basis_points_percentage(570), "5.7");
    assert_eq!(format_basis_points_percentage(576), "5.76");
    assert_eq!(parse_basis_points_percentage("5.76"), Some(576));
    assert_eq!(parse_basis_points_percentage(",5"), Some(50));
    assert_eq!(parse_basis_points_percentage("5."), Some(500));
    assert_eq!(parse_basis_points_percentage("5.123"), None);
    assert_eq!(parse_basis_points_percentage("42949673"), None);
}
