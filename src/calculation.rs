use crate::{
    annual_tax_for_income_profile, estimated_sgi_progress_for_income, monthly_deduction,
    public_pension_progress_for_income, AnnualIncomeProfile, AnnualTax, IncomeBasisEstimate,
    IncomePlan, TaxAgeGroup, TaxDeduction,
};

/// Default monthly salary used by the GUI. It is the highest whole monthly
/// amount whose annualized income stays below the 2026 state-tax breakpoint.
pub const DEFAULT_MONTHLY_INCOME: u32 = 660_400 / 12;
pub const DIVIDEND_TAX_PERCENT: u32 = 20;

/// How a percentage jämkning decision changes the projected ordinary tax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdjustmentCalibration {
    pub basis_income: u32,
    pub percent: u32,
    pub formula_tax_at_basis: u32,
    pub assumed_tax_at_basis: u32,
    pub implied_tax_adjustment: i64,
    pub projected_ordinary_tax: u32,
}

/// Complete tax projection for an income plan.
///
/// Keeping this model in the library ensures every consumer uses the same tax,
/// withholding, reconciliation, and income-basis rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calculation {
    pub monthly_income: u32,
    pub annual_income: u32,
    pub ordinary_income: u32,
    pub dividend_income: u32,
    pub table_deduction: TaxDeduction,
    pub annual_tax: AnnualTax,
    pub adjustment_calibration: Option<AdjustmentCalibration>,
    pub ordinary_final_tax: u32,
    pub dividend_tax: u32,
    pub total_tax: u32,
    pub withheld_tax: u32,
    pub regular_pension_premiums: u32,
    pub vacation_pension_premiums: u32,
    pub salary_exchange_sacrifice: u32,
    pub salary_exchange_pension_contributions: u32,
    pub employer_pension_contributions: u32,
    pub marginal_rate: f64,
    pub pension_progress: IncomeBasisEstimate,
    pub sgi_progress: IncomeBasisEstimate,
}

impl Calculation {
    pub fn new(table: u8, age_group: TaxAgeGroup, plan: &IncomePlan) -> Option<Self> {
        if !plan.is_valid() {
            return None;
        }

        let totals = plan.totals();
        let annual_income = totals.gross_income();
        let ordinary_income = totals.ordinary_income();
        let monthly_income = totals.monthly_taxable_income();
        let table_deduction = monthly_deduction(table, age_group.salary_column(), monthly_income)?;
        let annual_tax = annual_tax_for_income_profile(table, age_group, totals.annual_profile())?;
        let adjustment_calibration =
            match (plan.adjustment_percent, totals.adjustment_basis_work_income) {
                (Some(percent), basis_income) if basis_income > 0 => {
                    let basis_profile = AnnualIncomeProfile {
                        work_income: basis_income,
                        pension_income: 0,
                    };
                    let formula_tax_at_basis =
                        annual_tax_for_income_profile(table, age_group, basis_profile)?.total;
                    let assumed_tax_at_basis = percentage(basis_income, percent);
                    let implied_tax_adjustment =
                        i64::from(formula_tax_at_basis) - i64::from(assumed_tax_at_basis);
                    let projected_ordinary_tax =
                        (i64::from(annual_tax.total) - implied_tax_adjustment)
                            .clamp(0, i64::from(u32::MAX)) as u32;
                    Some(AdjustmentCalibration {
                        basis_income,
                        percent,
                        formula_tax_at_basis,
                        assumed_tax_at_basis,
                        implied_tax_adjustment,
                        projected_ordinary_tax,
                    })
                }
                _ => None,
            };
        let ordinary_final_tax = adjustment_calibration
            .map(|calibration| calibration.projected_ordinary_tax)
            .unwrap_or(annual_tax.total);
        let dividend_tax = percentage(totals.dividend_income, DIVIDEND_TAX_PERCENT);
        let total_tax = ordinary_final_tax.saturating_add(dividend_tax);
        let withheld_tax = plan.estimated_withholding(table, age_group).total;
        let upper_profile = AnnualIncomeProfile {
            work_income: totals.work_income.saturating_add(12_000),
            pension_income: totals.pension_income,
        };
        let upper_tax = annual_tax_for_income_profile(table, age_group, upper_profile)?.total;
        let marginal_rate = (f64::from(upper_tax) - f64::from(annual_tax.total)) * 100.0 / 12_000.0;

        Some(Self {
            monthly_income,
            annual_income,
            ordinary_income,
            dividend_income: totals.dividend_income,
            table_deduction,
            annual_tax,
            adjustment_calibration,
            ordinary_final_tax,
            dividend_tax,
            total_tax,
            withheld_tax,
            regular_pension_premiums: totals.regular_pension_premiums,
            vacation_pension_premiums: totals.vacation_pension_premiums,
            salary_exchange_sacrifice: totals.salary_exchange_sacrifice,
            salary_exchange_pension_contributions: totals.salary_exchange_pension_contributions,
            employer_pension_contributions: totals.total_employer_pension_contributions(),
            marginal_rate,
            pension_progress: public_pension_progress_for_income(totals.work_income),
            sgi_progress: estimated_sgi_progress_for_income(totals.sgi_annual_rate),
        })
    }

    pub fn table_reference_tax(self) -> u32 {
        match self.table_deduction {
            TaxDeduction::Amount(amount) => amount,
            TaxDeduction::Percent(percent) => percentage(self.monthly_income, percent),
        }
    }

    pub fn table_reference_net(self) -> u32 {
        self.monthly_income
            .saturating_sub(self.table_reference_tax())
    }

    pub fn annualized_table_reference_tax(self) -> u32 {
        self.table_reference_tax().saturating_mul(12)
    }

    pub fn effective_rate(self) -> f64 {
        if self.ordinary_income == 0 {
            0.0
        } else {
            f64::from(self.ordinary_final_tax) * 100.0 / f64::from(self.ordinary_income)
        }
    }

    pub const fn annual_net(self) -> u32 {
        self.annual_income.saturating_sub(self.total_tax)
    }

    pub const fn cash_after_withholding(self) -> u32 {
        self.annual_income.saturating_sub(self.withheld_tax)
    }

    pub fn tax_balance(self) -> i64 {
        i64::from(self.total_tax) - i64::from(self.withheld_tax)
    }
}

fn percentage(amount: u32, percent: u32) -> u32 {
    (u64::from(amount) * u64::from(percent) / 100).min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Date2026, IncomeKind, PayerRole, MAX_TAX_TABLE, MIN_TAX_TABLE};

    #[test]
    fn full_year_adjustment_basis_calibrates_the_partial_year_projection() {
        let mut plan = IncomePlan::with_monthly_salary(93_000);
        plan.adjustment_percent = Some(33);
        plan.entries[0].end = Date2026::new(10, 18);
        plan.entries[0].adjustment_applies = true;
        plan.entries[0].use_full_year_projection_as_adjustment_basis = true;

        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();
        let calibration = calculation.adjustment_calibration.unwrap();

        assert_eq!(calibration.basis_income, 1_116_000);
        assert_eq!(calibration.assumed_tax_at_basis, 368_280);
        assert_eq!(calibration.formula_tax_at_basis, 392_457);
        assert_eq!(calibration.implied_tax_adjustment, 24_177);
        assert_eq!(calculation.annual_tax.total, 275_457);
        assert_eq!(calculation.ordinary_final_tax, 251_280);
        assert_eq!(calculation.withheld_tax, 294_030);
        assert_eq!(calculation.tax_balance(), -42_750);
    }

    #[test]
    fn whole_percentage_table_can_leave_a_balance() {
        let plan = IncomePlan::with_monthly_salary(93_000);
        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.annual_tax.total, 392_457);
        assert_eq!(calculation.withheld_tax, 390_600);
        assert_eq!(calculation.annualized_table_reference_tax(), 390_600);
        assert_eq!(calculation.tax_balance(), 1_857);
    }

    #[test]
    fn detailed_income_scenario_drives_all_projection_inputs() {
        let mut plan = IncomePlan::with_annual_salary(0);
        let salary = &mut plan.entries[0];
        salary.kind = IncomeKind::MonthlySalary;
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
        pension.payer_role = PayerRole::Secondary;

        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();
        assert_eq!(calculation.ordinary_income, 1_521_028);
        assert_eq!(calculation.monthly_income, 126_752);
        assert!(calculation.withheld_tax > 0);
        assert_eq!(
            calculation.pension_progress,
            public_pension_progress_for_income(1_383_528)
        );
        assert_eq!(
            calculation.sgi_progress,
            estimated_sgi_progress_for_income(1_116_000)
        );
    }

    #[test]
    fn dividend_tax_and_marginal_rate_are_part_of_the_domain_projection() {
        let mut plan = IncomePlan::with_annual_salary(216_000);
        let dividend_id = plan.add_entry(IncomeKind::OwnCompanyDividend);
        plan.entries
            .iter_mut()
            .find(|entry| entry.id == dividend_id)
            .unwrap()
            .amount = 200_000;
        let calculation = Calculation::new(34, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.dividend_tax, 40_000);
        assert_eq!(calculation.total_tax, calculation.annual_tax.total + 40_000);
        let expected = f64::from(38_894 - 35_889) * 100.0 / 12_000.0;
        assert_eq!(calculation.marginal_rate, expected);
    }

    #[test]
    fn zero_income_and_every_published_table_are_supported() {
        let zero = IncomePlan::with_annual_salary(0);
        let calculation = Calculation::new(33, TaxAgeGroup::Under66AtYearStart, &zero).unwrap();
        assert_eq!(calculation.table_deduction, TaxDeduction::Amount(0));
        assert_eq!(calculation.total_tax, 0);
        assert_eq!(calculation.table_reference_net(), 0);
        assert_eq!(calculation.effective_rate(), 0.0);

        let plan = IncomePlan::with_annual_salary(420_000);
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            assert!(Calculation::new(table, TaxAgeGroup::Under66AtYearStart, &plan).is_some());
        }
    }

    #[test]
    fn default_income_stays_below_the_state_tax_breakpoint() {
        let plan = IncomePlan::with_monthly_salary(DEFAULT_MONTHLY_INCOME);
        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.monthly_income, 55_033);
        assert_eq!(calculation.annual_income, 660_396);
        assert_eq!(calculation.annual_tax.state_income_tax, 0);
        assert!(calculation.monthly_income * 12 <= 660_400);
        assert!((calculation.monthly_income + 1) * 12 > 660_400);
    }

    #[test]
    fn annual_salary_uses_one_twelfth_for_the_monthly_reference() {
        let plan = IncomePlan::with_annual_salary(420_011);
        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.monthly_income, 35_000);
        assert_eq!(calculation.annual_income, 420_011);
        assert_eq!(
            calculation.table_deduction,
            monthly_deduction(32, TaxAgeGroup::Under66AtYearStart.salary_column(), 35_000,)
                .unwrap()
        );
    }
}
