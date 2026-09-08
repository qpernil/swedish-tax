use crate::{DEFAULT_MONTHLY_INCOME, IncomePlan, MAX_TAX_TABLE, MIN_TAX_TABLE, TaxAgeGroup};

/// Application settings shared by native frontends.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PersistedAppState {
    pub table: u8,
    pub age_group: TaxAgeGroup,
    pub income_plan: IncomePlan,
}

impl PersistedAppState {
    pub fn new(table: u8, age_group: TaxAgeGroup, income_plan: IncomePlan) -> Self {
        Self {
            table,
            age_group,
            income_plan,
        }
    }

    /// Rejects unsupported tax tables.
    pub fn is_supported(&self) -> bool {
        (MIN_TAX_TABLE..=MAX_TAX_TABLE).contains(&self.table)
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
    use crate::{Date2026, IncomeKind};

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
    fn unsupported_tables_are_rejected() {
        let mut state = PersistedAppState::default();
        assert!(state.is_supported());
        state.table = MIN_TAX_TABLE - 1;
        assert!(!state.is_supported());
    }

    #[test]
    fn missing_required_plan_fields_are_rejected() {
        let encoded = ron::to_string(&PersistedAppState::default()).unwrap();
        let incomplete = encoded.replace("own_company_sourced:false,", "");
        assert_ne!(incomplete, encoded);
        assert!(ron::from_str::<PersistedAppState>(&incomplete).is_err());
    }
}
