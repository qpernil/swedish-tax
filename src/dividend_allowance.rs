/// 2027 3:12 parameters for qualified shares in closely held companies.
pub const DIVIDEND_BASIC_AMOUNT_2027: u32 = 333_600;
pub const DIVIDEND_WAGE_DEDUCTION_2027: u32 = 667_200;
pub const DIVIDEND_WAGE_ALLOWANCE_PERCENT: u32 = 50;
pub const DIVIDEND_WAGE_CAP_MULTIPLIER: u32 = 50;
pub const DIVIDEND_ACQUISITION_COST_THRESHOLD: u32 = 100_000;
pub const QUALIFIED_DIVIDEND_TAX_PERCENT: u32 = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DividendAllowanceInputs2027 {
    /// When true, the owner's marked 2026 cash salary is the complete wage basis.
    #[serde(default = "default_true")]
    pub one_person_company: bool,
    /// Hundredths of one percent; 10_000 means 100%.
    pub ownership_basis_points: u32,
    /// Sum of this owner's percentages in other companies with qualified shares.
    /// This can exceed 100% when the owner has shares in several companies.
    pub other_qualified_ownership_basis_points: u32,
    /// A spouse's ownership in this company, used for the mandatory joint wage calculation.
    pub spouse_ownership_basis_points: u32,
    /// Cash compensation paid by the company and qualifying subsidiaries in 2026.
    #[serde(default, alias = "company_cash_payroll_2025")]
    pub company_cash_payroll_2026: u32,
    /// Highest 2026 cash compensation paid to a related person by the company/group.
    #[serde(default, alias = "highest_related_cash_salary_2025")]
    pub highest_related_cash_salary_2026: u32,
    /// Acquisition cost for this owner's shares at the beginning of 2027.
    pub acquisition_cost: u32,
    /// Total 2027 interest rate: the 30 November 2026 government borrowing rate plus 9%.
    #[serde(default)]
    pub acquisition_cost_interest_basis_points: Option<u32>,
    /// Saved dividend allowance brought into income year 2027.
    pub saved_allowance: u32,
}

impl Default for DividendAllowanceInputs2027 {
    fn default() -> Self {
        Self {
            one_person_company: true,
            ownership_basis_points: 10_000,
            other_qualified_ownership_basis_points: 0,
            spouse_ownership_basis_points: 0,
            company_cash_payroll_2026: 0,
            highest_related_cash_salary_2026: 0,
            acquisition_cost: 0,
            acquisition_cost_interest_basis_points: None,
            saved_allowance: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DividendAllowanceIssue {
    OwnershipExceedsOneHundredPercent,
    SpouseOwnershipExceedsCompany,
    PersonalSalaryExceedsCompanyPayroll,
    MissingAcquisitionCostInterestRate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DividendAllowance2027 {
    pub basic_amount: u32,
    pub owner_cash_salary: u32,
    pub company_cash_payroll: u32,
    pub joint_wage_basis: u32,
    pub joint_wage_basis_after_deduction: u32,
    pub wage_allowance_before_cap: u32,
    pub wage_cap_salary: u32,
    pub wage_cap: u32,
    pub wage_allowance: u32,
    pub acquisition_cost_interest_basis: u32,
    pub acquisition_cost_interest: u32,
    pub saved_allowance: u32,
    pub total: u32,
}

impl DividendAllowanceInputs2027 {
    pub fn calculate(
        self,
        owner_cash_salary_2026: u32,
    ) -> Result<DividendAllowance2027, DividendAllowanceIssue> {
        if self.ownership_basis_points > 10_000 {
            return Err(DividendAllowanceIssue::OwnershipExceedsOneHundredPercent);
        }
        let joint_ownership = self
            .ownership_basis_points
            .saturating_add(self.spouse_ownership_basis_points);
        if joint_ownership > 10_000 {
            return Err(DividendAllowanceIssue::SpouseOwnershipExceedsCompany);
        }
        let company_cash_payroll = if self.one_person_company {
            owner_cash_salary_2026
        } else {
            self.company_cash_payroll_2026
        };
        let highest_related_cash_salary = if self.one_person_company {
            0
        } else {
            self.highest_related_cash_salary_2026
        };
        if owner_cash_salary_2026 > company_cash_payroll
            || highest_related_cash_salary > company_cash_payroll
        {
            return Err(DividendAllowanceIssue::PersonalSalaryExceedsCompanyPayroll);
        }
        let basic_denominator = self
            .ownership_basis_points
            .saturating_add(self.other_qualified_ownership_basis_points)
            .max(10_000);
        let basic_amount = proportion_floor(
            DIVIDEND_BASIC_AMOUNT_2027,
            self.ownership_basis_points,
            basic_denominator,
        );

        let joint_wage_basis = proportion_floor(company_cash_payroll, joint_ownership, 10_000);
        let joint_wage_basis_after_deduction =
            joint_wage_basis.saturating_sub(DIVIDEND_WAGE_DEDUCTION_2027);
        let joint_wage_allowance = percentage_floor(
            joint_wage_basis_after_deduction,
            DIVIDEND_WAGE_ALLOWANCE_PERCENT,
        );
        let wage_allowance_before_cap = if joint_ownership == 0 {
            0
        } else {
            proportion_floor(
                joint_wage_allowance,
                self.ownership_basis_points,
                joint_ownership,
            )
        };
        let wage_cap_salary = owner_cash_salary_2026.max(highest_related_cash_salary);
        let wage_cap = wage_cap_salary.saturating_mul(DIVIDEND_WAGE_CAP_MULTIPLIER);
        let wage_allowance = wage_allowance_before_cap.min(wage_cap);

        let acquisition_cost_interest_basis = self
            .acquisition_cost
            .saturating_sub(DIVIDEND_ACQUISITION_COST_THRESHOLD);
        let acquisition_cost_interest = if acquisition_cost_interest_basis == 0 {
            0
        } else if let Some(rate) = self.acquisition_cost_interest_basis_points {
            basis_points_floor(acquisition_cost_interest_basis, rate)
        } else {
            return Err(DividendAllowanceIssue::MissingAcquisitionCostInterestRate);
        };
        let total = basic_amount
            .saturating_add(wage_allowance)
            .saturating_add(acquisition_cost_interest)
            .saturating_add(self.saved_allowance);
        Ok(DividendAllowance2027 {
            basic_amount,
            owner_cash_salary: owner_cash_salary_2026,
            company_cash_payroll,
            joint_wage_basis,
            joint_wage_basis_after_deduction,
            wage_allowance_before_cap,
            wage_cap_salary,
            wage_cap,
            wage_allowance,
            acquisition_cost_interest_basis,
            acquisition_cost_interest,
            saved_allowance: self.saved_allowance,
            total,
        })
    }
}

const fn default_true() -> bool {
    true
}

impl DividendAllowance2027 {
    pub fn tax_at_twenty_percent(self) -> u32 {
        percentage_floor(self.total, QUALIFIED_DIVIDEND_TAX_PERCENT)
    }

    pub fn net_after_twenty_percent_tax(self) -> u32 {
        self.total.saturating_sub(self.tax_at_twenty_percent())
    }
}

fn proportion_floor(amount: u32, numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    (u64::from(amount)
        .saturating_mul(u64::from(numerator))
        .saturating_div(u64::from(denominator)))
    .min(u64::from(u32::MAX)) as u32
}

fn percentage_floor(amount: u32, percent: u32) -> u32 {
    proportion_floor(amount, percent, 100)
}

fn basis_points_floor(amount: u32, basis_points: u32) -> u32 {
    proportion_floor(amount, basis_points, 10_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_one_person_company_uses_2026_salary_for_2027_allowance() {
        let result = DividendAllowanceInputs2027::default()
            .calculate(800_000)
            .unwrap();

        assert_eq!(result.basic_amount, 333_600);
        assert_eq!(result.company_cash_payroll, 800_000);
        assert_eq!(result.joint_wage_basis_after_deduction, 132_800);
        assert_eq!(result.wage_allowance, 66_400);
        assert_eq!(result.total, 400_000);
    }

    #[test]
    fn salary_below_the_2027_deduction_adds_no_wage_allowance() {
        let result = DividendAllowanceInputs2027::default()
            .calculate(500_000)
            .unwrap();

        assert_eq!(result.wage_allowance, 0);
        assert_eq!(result.total, 333_600);
    }

    #[test]
    fn acquisition_cost_above_threshold_needs_the_later_2027_rate() {
        let inputs = DividendAllowanceInputs2027 {
            acquisition_cost: 250_000,
            ..Default::default()
        };

        assert_eq!(
            inputs.calculate(0),
            Err(DividendAllowanceIssue::MissingAcquisitionCostInterestRate)
        );

        let result = DividendAllowanceInputs2027 {
            acquisition_cost_interest_basis_points: Some(1_155),
            ..inputs
        }
        .calculate(0)
        .unwrap();
        assert_eq!(result.acquisition_cost_interest, 17_325);
    }

    #[test]
    fn basic_amount_is_proportionally_limited_across_multiple_companies() {
        let result = DividendAllowanceInputs2027 {
            ownership_basis_points: 2_500,
            other_qualified_ownership_basis_points: 17_500,
            ..Default::default()
        }
        .calculate(0)
        .unwrap();

        assert_eq!(result.basic_amount, 41_700);
    }

    #[test]
    fn spouse_wage_calculation_is_joint_then_allocated_by_ownership() {
        let result = DividendAllowanceInputs2027 {
            ownership_basis_points: 6_000,
            spouse_ownership_basis_points: 4_000,
            one_person_company: false,
            company_cash_payroll_2026: 4_000_000,
            ..Default::default()
        }
        .calculate(500_000)
        .unwrap();

        assert_eq!(result.wage_allowance_before_cap, 999_840);
        assert_eq!(result.wage_allowance, 999_840);
    }

    #[test]
    fn wage_allowance_is_capped_at_fifty_times_salary() {
        let result = DividendAllowanceInputs2027 {
            one_person_company: false,
            company_cash_payroll_2026: 10_000_000,
            ..Default::default()
        }
        .calculate(20_000)
        .unwrap();

        assert!(result.wage_allowance_before_cap > 1_000_000);
        assert_eq!(result.wage_cap, 1_000_000);
        assert_eq!(result.wage_allowance, 1_000_000);
    }

    #[test]
    fn highest_related_salary_can_set_the_wage_cap() {
        let result = DividendAllowanceInputs2027 {
            one_person_company: false,
            company_cash_payroll_2026: 10_000_000,
            highest_related_cash_salary_2026: 30_000,
            ..Default::default()
        }
        .calculate(10_000)
        .unwrap();

        assert_eq!(result.wage_cap_salary, 30_000);
        assert_eq!(result.wage_cap, 1_500_000);
        assert_eq!(result.wage_allowance, 1_500_000);
    }

    #[test]
    fn personal_salary_cannot_exceed_total_company_payroll() {
        let issue = DividendAllowanceInputs2027 {
            one_person_company: false,
            company_cash_payroll_2026: 500_000,
            ..Default::default()
        }
        .calculate(600_000)
        .unwrap_err();

        assert_eq!(
            issue,
            DividendAllowanceIssue::PersonalSalaryExceedsCompanyPayroll
        );
    }
}
