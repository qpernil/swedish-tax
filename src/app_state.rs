use crate::{DEFAULT_MONTHLY_INCOME, IncomePlan, MAX_TAX_TABLE, MIN_TAX_TABLE, TaxAgeGroup};

pub const PERSISTED_APP_STATE_VERSION: u32 = 1;

/// Versioned application settings shared by native frontends.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PersistedAppState {
    pub version: u32,
    pub table: u8,
    pub age_group: TaxAgeGroup,
    pub income_plan: IncomePlan,
}

impl PersistedAppState {
    pub fn new(table: u8, age_group: TaxAgeGroup, income_plan: IncomePlan) -> Self {
        Self {
            version: PERSISTED_APP_STATE_VERSION,
            table,
            age_group,
            income_plan,
        }
    }

    /// Rejects states from incompatible schema versions or unsupported tables.
    pub fn is_supported(&self) -> bool {
        self.version == PERSISTED_APP_STATE_VERSION
            && (MIN_TAX_TABLE..=MAX_TAX_TABLE).contains(&self.table)
    }
}

impl Default for PersistedAppState {
    fn default() -> Self {
        Self::new(
            32,
            TaxAgeGroup::Under66AtYearStart,
            IncomePlan::with_monthly_salary(DEFAULT_MONTHLY_INCOME),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DEFAULT_VACATION_COMPENSATION_RATE_BASIS_POINTS, Date2026, IncomeKind, VacationCompensation,
    };

    #[test]
    fn complete_app_state_round_trips_through_serde() {
        let mut plan = IncomePlan::with_monthly_salary(93_000);
        plan.adjustment_percent = Some(33);
        plan.entries[0].end = Date2026::new(10, 18);
        plan.entries[0].adjustment_applies = true;
        plan.entries[0].own_company_sourced = true;
        plan.entries[0].actual_withholding = Some(271_234);
        plan.entries[0].additional_withholding_per_payment = Some(1_500);
        plan.dividend_allowance.one_person_company = false;
        plan.dividend_allowance.company_cash_payroll_2026 = 4_000_000;
        let pension_id = plan.add_entry(IncomeKind::MonthlyOccupationalPension);
        plan.entries
            .iter_mut()
            .find(|entry| entry.id == pension_id)
            .unwrap()
            .amount = 27_500;
        let expected = PersistedAppState::new(34, TaxAgeGroup::AtLeast66AtYearStart, plan);

        let encoded = ron::to_string(&expected).unwrap();
        let mut restored: PersistedAppState = ron::from_str(&encoded).unwrap();

        assert_eq!(restored, expected);
        assert_eq!(restored.income_plan.add_entry(IncomeKind::AnnualSalary), 3);
    }

    #[test]
    fn unsupported_versions_and_tables_are_rejected() {
        let mut state = PersistedAppState::default();
        assert!(state.is_supported());
        state.version += 1;
        assert!(!state.is_supported());
        state = PersistedAppState::default();
        state.table = MIN_TAX_TABLE - 1;
        assert!(!state.is_supported());
    }

    #[test]
    fn state_saved_before_new_optional_plan_fields_were_added_still_loads() {
        let encoded = ron::to_string(&PersistedAppState::default()).unwrap();
        let legacy = encoded
            .replace("own_company_sourced:false,", "")
            .replace("actual_withholding:None,", "")
            .replace("use_annual_daily_rate_for_partial_months:false,", "")
            .replace(
                "dividend_allowance:(one_person_company:true,ownership_basis_points:10000,other_qualified_ownership_basis_points:0,spouse_ownership_basis_points:0,company_cash_payroll_2026:0,highest_related_cash_salary_2026:0,acquisition_cost:0,acquisition_cost_interest_basis_points:None,saved_allowance:0),",
                "",
            );
        assert_ne!(legacy, encoded);
        assert!(!legacy.contains("own_company_sourced"));
        assert!(!legacy.contains("dividend_allowance"));

        let restored: PersistedAppState = ron::from_str(&legacy).unwrap();

        assert_eq!(restored.income_plan.entries[0].actual_withholding, None);
        assert!(!restored.income_plan.entries[0].own_company_sourced);
        assert!(!restored.income_plan.entries[0].use_annual_daily_rate_for_partial_months);
        assert_eq!(restored.income_plan.dividend_allowance, Default::default());
    }

    #[test]
    fn vacation_saved_before_rate_field_uses_default_rate() {
        let mut state = PersistedAppState::default();
        let entry = &mut state.income_plan.entries[0];
        entry.amount = 93_000;
        entry.end = Date2026::new(10, 18);
        entry.vacation_compensation =
            Some(VacationCompensation::suggested(30, entry.start, entry.end));
        let encoded = ron::to_string(&state).unwrap();
        let legacy = encoded.replace(
            &format!("rate_basis_points:{DEFAULT_VACATION_COMPENSATION_RATE_BASIS_POINTS},"),
            "",
        );
        assert_ne!(legacy, encoded);

        let restored: PersistedAppState = ron::from_str(&legacy).unwrap();
        let vacation = restored.income_plan.entries[0]
            .vacation_compensation
            .unwrap();

        assert_eq!(
            vacation.rate_basis_points,
            DEFAULT_VACATION_COMPENSATION_RATE_BASIS_POINTS
        );
        assert_eq!(vacation.amount(93_000), 120_528);
    }

    #[test]
    fn salary_exchange_saved_before_previous_year_basis_still_loads() {
        let mut state = PersistedAppState::default();
        let entry_id = state.income_plan.add_entry(IncomeKind::OneTimeSalary);
        let entry = state
            .income_plan
            .entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .unwrap();
        entry.salary_exchange = Some(crate::SalaryExchange::new());
        let encoded = ron::to_string(&state).unwrap();
        let legacy = encoded
            .replace(",previous_year_pension_salary_basis:None", "")
            .replace(",pension_and_insurance_costs_before_exchange:None", "");
        assert_ne!(legacy, encoded);

        let restored: PersistedAppState = ron::from_str(&legacy).unwrap();
        let exchange = restored
            .income_plan
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .unwrap()
            .salary_exchange
            .unwrap();

        assert_eq!(exchange.previous_year_pension_salary_basis, None);
        assert_eq!(exchange.pension_and_insurance_costs_before_exchange, None);
    }
}
