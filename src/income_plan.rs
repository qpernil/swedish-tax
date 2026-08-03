use crate::{
    monthly_deduction, one_time_withholding_rate, AnnualIncomeProfile, DividendAllowance2027,
    DividendAllowanceInputs2027, DividendAllowanceIssue, TaxAgeGroup, TaxColumn, TaxDeduction,
};

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct Date2026 {
    pub month: u8,
    pub day: u8,
}

impl Date2026 {
    pub const fn new(month: u8, day: u8) -> Self {
        Self { month, day }
    }

    pub const fn days_in_month(month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    pub fn clamped(self) -> Self {
        let month = self.month.clamp(1, 12);
        let day = self.day.clamp(1, Self::days_in_month(month));
        Self { month, day }
    }

    pub fn ordinal(self) -> u16 {
        const STARTS: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let date = self.clamped();
        STARTS[date.month as usize - 1] + u16::from(date.day)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum IncomeKind {
    AnnualSalary,
    MonthlySalary,
    OneTimeSalary,
    MonthlyOccupationalPension,
    AnnualOccupationalPension,
    OwnCompanyDividend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncomeTaxCategory {
    Work,
    Pension,
    Dividend,
}

impl IncomeKind {
    pub const ALL: [Self; 6] = [
        Self::AnnualSalary,
        Self::MonthlySalary,
        Self::OneTimeSalary,
        Self::MonthlyOccupationalPension,
        Self::AnnualOccupationalPension,
        Self::OwnCompanyDividend,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::AnnualSalary => "Ordinary salary — annual total",
            Self::MonthlySalary => "Salary — monthly over a period",
            Self::OneTimeSalary => "One-time salary / termination payment",
            Self::MonthlyOccupationalPension => "Tjänstepension — monthly over a period",
            Self::AnnualOccupationalPension => "Tjänstepension — annual total",
            Self::OwnCompanyDividend => "Dividend from own AB — 20% within gränsbelopp",
        }
    }

    pub const fn is_monthly(self) -> bool {
        matches!(self, Self::MonthlySalary | Self::MonthlyOccupationalPension)
    }

    pub const fn is_dividend(self) -> bool {
        matches!(self, Self::OwnCompanyDividend)
    }

    pub const fn is_salary(self) -> bool {
        matches!(
            self,
            Self::AnnualSalary | Self::MonthlySalary | Self::OneTimeSalary
        )
    }

    pub const fn is_pension(self) -> bool {
        matches!(
            self,
            Self::MonthlyOccupationalPension | Self::AnnualOccupationalPension
        )
    }

    pub const fn tax_category(self) -> IncomeTaxCategory {
        if self.is_dividend() {
            IncomeTaxCategory::Dividend
        } else if self.is_pension() {
            IncomeTaxCategory::Pension
        } else {
            IncomeTaxCategory::Work
        }
    }

    pub const fn is_pgi_eligible(self) -> bool {
        matches!(
            self,
            Self::AnnualSalary | Self::MonthlySalary | Self::OneTimeSalary
        )
    }

    pub const fn is_sgi_eligible(self) -> bool {
        matches!(self, Self::AnnualSalary | Self::MonthlySalary)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PayerRole {
    Main,
    Secondary,
}

/// 2026 ITP 1-equivalent benchmark used for an individually agreed
/// occupational pension when no collective-agreement formula is available.
pub const REGULAR_PENSION_MONTHLY_THRESHOLD: u32 = 52_125;
pub const REGULAR_PENSION_LOWER_RATE_BASIS_POINTS: u32 = 450;
pub const REGULAR_PENSION_UPPER_RATE_BASIS_POINTS: u32 = 3_000;
pub const EMPLOYER_PENSION_ALLOWANCE_RATE_BASIS_POINTS: u32 = 3_500;
pub const EMPLOYER_PENSION_ALLOWANCE_MAXIMUM: u32 = 592_000;
pub const DEFAULT_SALARY_EXCHANGE_UPLIFT_BASIS_POINTS: u32 = 576;
pub const SECONDARY_WITHHOLDING_PERCENT: u32 = 30;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RegularPensionPremium {
    /// Actual monthly premium, when it differs from the benchmark calculation.
    pub monthly_override: Option<u32>,
}

impl RegularPensionPremium {
    pub fn benchmark_monthly(monthly_salary: u32) -> u32 {
        let lower = monthly_salary.min(REGULAR_PENSION_MONTHLY_THRESHOLD);
        let upper = monthly_salary.saturating_sub(REGULAR_PENSION_MONTHLY_THRESHOLD);
        let numerator = u64::from(lower)
            .saturating_mul(u64::from(REGULAR_PENSION_LOWER_RATE_BASIS_POINTS))
            .saturating_add(
                u64::from(upper).saturating_mul(u64::from(REGULAR_PENSION_UPPER_RATE_BASIS_POINTS)),
            );
        ((numerator + 5_000) / 10_000).min(u64::from(u32::MAX)) as u32
    }

    pub fn monthly_amount(self, monthly_salary: u32) -> u32 {
        self.monthly_override
            .unwrap_or_else(|| Self::benchmark_monthly(monthly_salary))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SalaryExchange {
    /// Gross one-time salary forgone by the employee.
    pub sacrificed_salary: u32,
    pub employer_adds_uplift: bool,
    /// Hundredths of one percent; 576 means 5.76%.
    pub uplift_basis_points: u32,
}

impl SalaryExchange {
    pub const fn new() -> Self {
        Self {
            sacrificed_salary: 0,
            employer_adds_uplift: true,
            uplift_basis_points: DEFAULT_SALARY_EXCHANGE_UPLIFT_BASIS_POINTS,
        }
    }

    pub fn pension_contribution(self) -> u32 {
        if self.employer_adds_uplift {
            basis_points_rounded(
                self.sacrificed_salary,
                10_000_u32.saturating_add(self.uplift_basis_points),
            )
        } else {
            self.sacrificed_salary
        }
    }

    pub fn allowance_ceiling(pension_salary_basis: u32) -> u32 {
        basis_points_rounded(
            pension_salary_basis,
            EMPLOYER_PENSION_ALLOWANCE_RATE_BASIS_POINTS,
        )
        .min(EMPLOYER_PENSION_ALLOWANCE_MAXIMUM)
    }

    pub fn maximum_sacrifice(
        self,
        payment_amount: u32,
        pension_salary_basis_before: u32,
        pension_contributions_before: u32,
        payment_is_pensionable: bool,
    ) -> u32 {
        let mut low = 0_u32;
        let mut high = payment_amount;
        while low < high {
            let candidate_sacrifice = low + (high - low).div_ceil(2);
            let pension_salary_basis_after = if payment_is_pensionable {
                pension_salary_basis_before.saturating_sub(candidate_sacrifice)
            } else {
                pension_salary_basis_before
            };
            let ceiling = Self::allowance_ceiling(pension_salary_basis_after);
            let mut candidate = self;
            candidate.sacrificed_salary = candidate_sacrifice;
            let valid = pension_contributions_before
                .saturating_add(candidate.pension_contribution())
                <= ceiling;
            if valid {
                low = candidate_sacrifice;
            } else {
                high = candidate_sacrifice - 1;
            }
        }
        low
    }
}

impl Default for SalaryExchange {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct IncomeEntry {
    pub id: u64,
    pub description: String,
    pub kind: IncomeKind,
    /// Annual total, monthly amount, or one-time amount depending on `kind`.
    pub amount: u32,
    pub start: Date2026,
    pub end: Date2026,
    pub payer_role: PayerRole,
    /// Whether this cash salary is paid by the owner's company or qualifying group.
    /// For 2026 income rows it feeds the following year's 3:12 wage basis.
    #[serde(default)]
    pub own_company_sourced: bool,
    pub adjustment_applies: bool,
    /// Use this recurring salary's full-year projection as the income basis
    /// behind a percentage jämkning decision.
    pub use_full_year_projection_as_adjustment_basis: bool,
    pub custom_withholding_percent: Option<u32>,
    /// Total tax actually withheld for this income row. When present, this
    /// takes precedence over every estimated withholding rule.
    #[serde(default)]
    pub actual_withholding: Option<u32>,
    pub vacation_compensation: Option<VacationCompensation>,
    pub regular_pension_premium: Option<RegularPensionPremium>,
    pub salary_exchange: Option<SalaryExchange>,
    /// Whether cash from this entry is part of the employer's current-year
    /// occupational-pension salary basis.
    pub included_in_pension_salary_basis: bool,
}

impl IncomeEntry {
    pub fn new(id: u64, kind: IncomeKind) -> Self {
        let regular_pension_premium =
            matches!(kind, IncomeKind::AnnualSalary | IncomeKind::MonthlySalary)
                .then_some(RegularPensionPremium::default());
        Self {
            id,
            description: String::new(),
            kind,
            amount: 0,
            start: Date2026::new(1, 1),
            end: Date2026::new(12, 31),
            payer_role: PayerRole::Main,
            own_company_sourced: false,
            adjustment_applies: false,
            use_full_year_projection_as_adjustment_basis: false,
            custom_withholding_percent: None,
            actual_withholding: None,
            vacation_compensation: None,
            regular_pension_premium,
            salary_exchange: None,
            included_in_pension_salary_basis: matches!(
                kind,
                IncomeKind::AnnualSalary | IncomeKind::MonthlySalary
            ),
        }
    }

    /// Changes the income kind and resets fields that are not meaningful for
    /// the new kind. UI clients should use this instead of reproducing these
    /// domain invariants.
    pub fn set_kind(&mut self, kind: IncomeKind, adjustment_available: bool) {
        if self.kind == kind {
            return;
        }
        let previous_kind = self.kind;
        self.kind = kind;
        self.included_in_pension_salary_basis =
            matches!(kind, IncomeKind::AnnualSalary | IncomeKind::MonthlySalary);
        if matches!(kind, IncomeKind::AnnualSalary | IncomeKind::MonthlySalary)
            && self.regular_pension_premium.is_none()
        {
            self.regular_pension_premium = Some(RegularPensionPremium::default());
        }
        if kind != IncomeKind::OneTimeSalary {
            self.salary_exchange = None;
        }
        if kind != IncomeKind::MonthlySalary {
            self.vacation_compensation = None;
        }
        if !matches!(kind, IncomeKind::AnnualSalary | IncomeKind::MonthlySalary) {
            self.use_full_year_projection_as_adjustment_basis = false;
        }
        if kind.is_dividend() {
            self.adjustment_applies = false;
        } else if previous_kind.is_dividend() && self.payer_role == PayerRole::Main {
            self.adjustment_applies = adjustment_available;
        }
        if !kind.is_salary() {
            self.own_company_sourced = false;
        }
    }

    /// Changes the payer role and applies the plan's default jämkning choice.
    /// The user can still override `adjustment_applies` after changing the role.
    pub fn set_payer_role(&mut self, payer_role: PayerRole, adjustment_available: bool) {
        if self.payer_role == payer_role {
            return;
        }
        self.payer_role = payer_role;
        self.adjustment_applies =
            adjustment_available && payer_role == PayerRole::Main && !self.kind.is_dividend();
    }

    pub fn set_custom_withholding_enabled(&mut self, enabled: bool) {
        if enabled == self.custom_withholding_percent.is_some() {
            return;
        }
        self.custom_withholding_percent = enabled.then_some(30);
    }

    pub fn set_actual_withholding_enabled(&mut self, enabled: bool) {
        if enabled == self.actual_withholding.is_some() {
            return;
        }
        self.actual_withholding = enabled.then_some(0);
    }

    pub fn annual_amount(&self) -> u32 {
        if self.kind.is_monthly() {
            (1..=12)
                .map(|month| self.amount_for_month(month))
                .fold(0_u32, u32::saturating_add)
        } else {
            self.amount
        }
    }

    pub fn amount_for_month(&self, month: u8) -> u32 {
        if !self.kind.is_monthly() || self.start > self.end || !(1..=12).contains(&month) {
            return 0;
        }
        let start = self.start.clamped();
        let end = self.end.clamped();
        if month < start.month || month > end.month {
            return 0;
        }
        let first_day = if month == start.month { start.day } else { 1 };
        let last_day = if month == end.month {
            end.day
        } else {
            Date2026::days_in_month(month)
        };
        let active_days = u32::from(last_day.saturating_sub(first_day) + 1);
        self.amount.saturating_mul(active_days) / u32::from(Date2026::days_in_month(month))
    }

    pub fn is_valid(&self) -> bool {
        !self.kind.is_monthly() || self.start.clamped() <= self.end.clamped()
    }

    pub fn total_annual_amount(&self) -> u32 {
        self.annual_amount()
            .saturating_add(self.vacation_compensation_amount())
            .saturating_sub(self.salary_exchange_sacrifice())
    }

    pub fn pgi_eligible_income(&self) -> Option<u32> {
        self.kind
            .is_pgi_eligible()
            .then(|| self.total_annual_amount())
    }

    pub fn sgi_annual_rate(&self) -> Option<u32> {
        match self.kind {
            IncomeKind::AnnualSalary => Some(self.total_annual_amount()),
            IncomeKind::MonthlySalary => Some(self.amount.saturating_mul(12)),
            _ => None,
        }
    }

    pub fn total_employer_pension_contribution(&self) -> u32 {
        self.regular_pension_premium_amount()
            .saturating_add(self.vacation_pension_premium_amount())
            .saturating_add(self.salary_exchange_pension_contribution())
    }

    pub fn regular_pension_benchmark_monthly(&self) -> Option<u32> {
        match self.kind {
            IncomeKind::AnnualSalary => {
                Some(RegularPensionPremium::benchmark_monthly(self.amount / 12))
            }
            IncomeKind::MonthlySalary => {
                Some(RegularPensionPremium::benchmark_monthly(self.amount))
            }
            _ => None,
        }
    }

    pub fn full_year_adjustment_basis_amount(&self) -> u32 {
        if !self.use_full_year_projection_as_adjustment_basis {
            return 0;
        }
        match self.kind {
            IncomeKind::MonthlySalary => self.amount.saturating_mul(12),
            IncomeKind::AnnualSalary => self.amount,
            _ => 0,
        }
    }

    pub fn vacation_compensation_amount(&self) -> u32 {
        if self.kind != IncomeKind::MonthlySalary {
            return 0;
        }
        self.vacation_compensation
            .map(|vacation| vacation.amount(self.amount))
            .unwrap_or(0)
    }

    pub fn regular_pension_premium_amount(&self) -> u32 {
        let Some(premium) = self.regular_pension_premium else {
            return 0;
        };
        match self.kind {
            IncomeKind::AnnualSalary => premium.monthly_amount(self.amount / 12).saturating_mul(12),
            IncomeKind::MonthlySalary => {
                let monthly_premium = premium.monthly_amount(self.amount);
                (1..=12)
                    .map(|month| self.prorated_monthly_value(month, monthly_premium))
                    .fold(0_u32, u32::saturating_add)
            }
            _ => 0,
        }
    }

    pub fn vacation_pension_premium_amount(&self) -> u32 {
        let Some(vacation) = self.vacation_compensation else {
            return 0;
        };
        if self.kind != IncomeKind::MonthlySalary || !vacation.included_in_pension_salary_basis {
            return 0;
        }
        vacation.pension_premium_override.unwrap_or_else(|| {
            RegularPensionPremium::benchmark_monthly(
                self.amount
                    .saturating_add(self.vacation_compensation_amount()),
            )
            .saturating_sub(RegularPensionPremium::benchmark_monthly(self.amount))
        })
    }

    pub fn pension_salary_basis_amount(&self) -> u32 {
        let regular = if self.included_in_pension_salary_basis {
            self.annual_amount()
                .saturating_sub(self.salary_exchange_sacrifice())
        } else {
            0
        };
        let vacation = self
            .vacation_compensation
            .filter(|vacation| vacation.included_in_pension_salary_basis)
            .map(|_| self.vacation_compensation_amount())
            .unwrap_or(0);
        regular.saturating_add(vacation)
    }

    pub fn salary_exchange_sacrifice(&self) -> u32 {
        if self.kind != IncomeKind::OneTimeSalary {
            return 0;
        }
        self.salary_exchange
            .map(|exchange| exchange.sacrificed_salary.min(self.amount))
            .unwrap_or(0)
    }

    pub fn salary_exchange_pension_contribution(&self) -> u32 {
        if self.kind != IncomeKind::OneTimeSalary {
            return 0;
        }
        self.salary_exchange
            .map(|mut exchange| {
                exchange.sacrificed_salary = exchange.sacrificed_salary.min(self.amount);
                exchange.pension_contribution()
            })
            .unwrap_or(0)
    }

    fn prorated_monthly_value(&self, month: u8, monthly_value: u32) -> u32 {
        if self.start > self.end || !(1..=12).contains(&month) {
            return 0;
        }
        let start = self.start.clamped();
        let end = self.end.clamped();
        if month < start.month || month > end.month {
            return 0;
        }
        let first_day = if month == start.month { start.day } else { 1 };
        let last_day = if month == end.month {
            end.day
        } else {
            Date2026::days_in_month(month)
        };
        let active_days = u32::from(last_day.saturating_sub(first_day) + 1);
        monthly_value.saturating_mul(active_days) / u32::from(Date2026::days_in_month(month))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct VacationCompensation {
    pub annual_entitlement_days: u32,
    pub payout_days: u32,
    pub included_in_pension_salary_basis: bool,
    /// Actual one-time employer premium attributable to the vacation payout.
    pub pension_premium_override: Option<u32>,
}

impl VacationCompensation {
    pub fn suggested(annual_entitlement_days: u32, start: Date2026, end: Date2026) -> Self {
        Self {
            annual_entitlement_days,
            payout_days: Self::suggested_days(annual_entitlement_days, start, end),
            included_in_pension_salary_basis: true,
            pension_premium_override: None,
        }
    }

    pub fn suggested_days(annual_entitlement_days: u32, start: Date2026, end: Date2026) -> u32 {
        let start = start.clamped();
        let end = end.clamped();
        if start > end {
            return 0;
        }
        let employment_days = u32::from(end.ordinal() - start.ordinal() + 1);
        annual_entitlement_days
            .saturating_mul(employment_days)
            .saturating_add(364)
            / 365
    }

    /// Statutory same-pay estimate: monthly salary / 21 plus 0.43% per day.
    pub fn amount(self, monthly_salary: u32) -> u32 {
        const DENOMINATOR: u64 = 21 * 10_000;
        const NUMERATOR_PER_DAY: u64 = 10_000 + 43 * 21;
        let numerator = u64::from(monthly_salary)
            .saturating_mul(u64::from(self.payout_days))
            .saturating_mul(NUMERATOR_PER_DAY);
        ((numerator + DENOMINATOR / 2) / DENOMINATOR).min(u64::from(u32::MAX)) as u32
    }

    pub fn amount_per_day(monthly_salary: u32) -> f64 {
        f64::from(monthly_salary) / 21.0 + f64::from(monthly_salary) * 0.0043
    }

    pub fn additional_benchmark_pension_premium(self, monthly_salary: u32) -> u32 {
        RegularPensionPremium::benchmark_monthly(
            monthly_salary.saturating_add(self.amount(monthly_salary)),
        )
        .saturating_sub(RegularPensionPremium::benchmark_monthly(monthly_salary))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct IncomePlan {
    pub entries: Vec<IncomeEntry>,
    pub adjustment_percent: Option<u32>,
    #[serde(default)]
    pub dividend_allowance: DividendAllowanceInputs2027,
    next_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncomePlanValidationIssue {
    InvalidPaymentPeriod { entry_id: u64 },
    SalaryExchangeExceedsAllowance { entry_id: u64, maximum: u32 },
}

impl IncomePlan {
    pub fn with_annual_salary(amount: u32) -> Self {
        let mut entry = IncomeEntry::new(1, IncomeKind::AnnualSalary);
        entry.description = "Ordinary income".to_owned();
        entry.amount = amount;
        Self {
            entries: vec![entry],
            adjustment_percent: None,
            dividend_allowance: DividendAllowanceInputs2027::default(),
            next_id: 2,
        }
    }

    pub fn with_monthly_salary(amount: u32) -> Self {
        let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
        entry.description = "Ordinary income".to_owned();
        entry.amount = amount;
        Self {
            entries: vec![entry],
            adjustment_percent: None,
            dividend_allowance: DividendAllowanceInputs2027::default(),
            next_id: 2,
        }
    }

    pub fn add_entry(&mut self, kind: IncomeKind) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let mut entry = IncomeEntry::new(id, kind);
        entry.adjustment_applies = self.adjustment_percent.is_some() && !kind.is_dividend();
        self.entries.push(entry);
        id
    }

    /// Enables or disables a percentage jämkning decision. When enabled for
    /// the first time, non-dividend main payers use it by default.
    pub fn set_adjustment_enabled(&mut self, enabled: bool) {
        if enabled == self.adjustment_percent.is_some() {
            return;
        }
        self.adjustment_percent = enabled.then_some(30);
        for entry in &mut self.entries {
            entry.adjustment_applies =
                enabled && entry.payer_role == PayerRole::Main && !entry.kind.is_dividend();
        }
    }

    pub fn remove_entry(&mut self, id: u64) {
        self.entries.retain(|entry| entry.id != id);
        if self.entries.is_empty() {
            self.add_entry(IncomeKind::AnnualSalary);
        }
    }

    pub fn validation_issue(&self) -> Option<IncomePlanValidationIssue> {
        for entry in &self.entries {
            if !entry.is_valid() {
                return Some(IncomePlanValidationIssue::InvalidPaymentPeriod {
                    entry_id: entry.id,
                });
            }
        }
        for entry in &self.entries {
            let Some(exchange) = entry.salary_exchange else {
                continue;
            };
            let Some(allowance) = self.salary_exchange_allowance(entry.id) else {
                continue;
            };
            if exchange.sacrificed_salary > allowance.maximum_sacrifice {
                return Some(IncomePlanValidationIssue::SalaryExchangeExceedsAllowance {
                    entry_id: entry.id,
                    maximum: allowance.maximum_sacrifice,
                });
            }
        }
        None
    }

    pub fn is_valid(&self) -> bool {
        self.validation_issue().is_none()
    }

    /// Whether the plan can be represented by the GUI's simple monthly table
    /// reference without hiding payer or period differences.
    pub fn has_uniform_monthly_table_reference(&self) -> bool {
        let [entry] = self.entries.as_slice() else {
            return false;
        };
        if entry.payer_role != PayerRole::Main
            || entry.adjustment_applies
            || entry.custom_withholding_percent.is_some()
            || entry.actual_withholding.is_some()
            || entry.vacation_compensation_amount() > 0
        {
            return false;
        }
        match entry.kind {
            IncomeKind::AnnualSalary => true,
            IncomeKind::MonthlySalary => {
                entry.start.clamped() == Date2026::new(1, 1)
                    && entry.end.clamped() == Date2026::new(12, 31)
            }
            _ => false,
        }
    }

    pub fn salary_exchange_context(&self, entry_id: u64) -> Option<SalaryExchangeContext> {
        let entry = self.entries.iter().find(|entry| entry.id == entry_id)?;
        let totals = self.totals();
        Some(SalaryExchangeContext {
            regular_pension_premiums: totals.regular_pension_premiums,
            vacation_pension_premiums: totals.vacation_pension_premiums,
            other_exchange_contributions: totals
                .salary_exchange_pension_contributions
                .saturating_sub(entry.salary_exchange_pension_contribution()),
            other_pension_salary_basis: totals
                .pension_salary_basis
                .saturating_sub(entry.pension_salary_basis_amount()),
        })
    }

    pub fn salary_exchange_allowance(&self, entry_id: u64) -> Option<SalaryExchangeAllowance> {
        let entry = self.entries.iter().find(|entry| entry.id == entry_id)?;
        let exchange = entry.salary_exchange?;
        Some(self.salary_exchange_context(entry_id)?.allowance_for(
            entry.amount,
            entry.included_in_pension_salary_basis,
            exchange,
        ))
    }

    pub fn totals(&self) -> IncomePlanTotals {
        let mut totals = IncomePlanTotals::default();
        for entry in &self.entries {
            let amount = entry.total_annual_amount();
            totals.regular_pension_premiums = totals
                .regular_pension_premiums
                .saturating_add(entry.regular_pension_premium_amount());
            totals.vacation_pension_premiums = totals
                .vacation_pension_premiums
                .saturating_add(entry.vacation_pension_premium_amount());
            totals.pension_salary_basis = totals
                .pension_salary_basis
                .saturating_add(entry.pension_salary_basis_amount());
            totals.adjustment_basis_work_income = totals
                .adjustment_basis_work_income
                .saturating_add(entry.full_year_adjustment_basis_amount());
            totals.salary_exchange_sacrifice = totals
                .salary_exchange_sacrifice
                .saturating_add(entry.salary_exchange_sacrifice());
            totals.salary_exchange_pension_contributions = totals
                .salary_exchange_pension_contributions
                .saturating_add(entry.salary_exchange_pension_contribution());
            match entry.kind.tax_category() {
                IncomeTaxCategory::Work => {
                    totals.work_income = totals.work_income.saturating_add(amount)
                }
                IncomeTaxCategory::Pension => {
                    totals.pension_income = totals.pension_income.saturating_add(amount)
                }
                IncomeTaxCategory::Dividend => {
                    totals.dividend_income = totals.dividend_income.saturating_add(amount)
                }
            }
            if let Some(sgi_annual_rate) = entry.sgi_annual_rate() {
                totals.sgi_annual_rate = totals.sgi_annual_rate.saturating_add(sgi_annual_rate);
            }
        }
        totals
    }

    /// 2026 cash salary marked as paid by the owner's company/group and used
    /// in the preliminary 2027 gränsbelopp.
    pub fn own_company_sourced_work_income(&self) -> u32 {
        self.entries
            .iter()
            .filter(|entry| entry.kind.is_salary() && entry.own_company_sourced)
            .map(IncomeEntry::total_annual_amount)
            .fold(0_u32, u32::saturating_add)
    }

    /// Preliminary 2027 allowance using marked 2026 own-company cash salary.
    pub fn dividend_allowance_2027(&self) -> Result<DividendAllowance2027, DividendAllowanceIssue> {
        self.dividend_allowance
            .calculate(self.own_company_sourced_work_income())
    }

    pub fn estimated_withholding(&self, table: u8, age_group: TaxAgeGroup) -> WithholdingSummary {
        let totals = self.totals();
        let mut entries = Vec::with_capacity(self.entries.len());
        let mut total = 0_u32;
        for entry in &self.entries {
            let gross = entry.total_annual_amount();
            let (withheld, regular_withheld, supplemental_withheld, rule) =
                self.entry_withholding(entry, gross, totals, table, age_group);
            total = total.saturating_add(withheld);
            entries.push(EntryWithholding {
                entry_id: entry.id,
                gross,
                withheld,
                regular_withheld,
                supplemental_withheld,
                rule,
            });
        }
        WithholdingSummary { total, entries }
    }

    fn entry_withholding(
        &self,
        entry: &IncomeEntry,
        gross: u32,
        totals: IncomePlanTotals,
        table: u8,
        age_group: TaxAgeGroup,
    ) -> (u32, u32, u32, AppliedWithholding) {
        if let Some(withheld) = entry.actual_withholding {
            return (withheld, withheld, 0, AppliedWithholding::ActualAmount);
        }
        if entry.kind.is_dividend() {
            return (0, 0, 0, AppliedWithholding::None);
        }
        if let Some(percent) = entry.custom_withholding_percent {
            let withheld = percentage(gross, percent);
            return (
                withheld,
                withheld,
                0,
                AppliedWithholding::CustomPercent(percent),
            );
        }
        if entry.adjustment_applies {
            if let Some(percent) = self.adjustment_percent {
                let withheld = percentage(gross, percent);
                return (
                    withheld,
                    withheld,
                    0,
                    AppliedWithholding::AdjustmentPercent(percent),
                );
            }
        }
        if entry.payer_role == PayerRole::Secondary {
            let withheld = percentage(gross, SECONDARY_WITHHOLDING_PERCENT);
            return (withheld, withheld, 0, AppliedWithholding::Secondary30);
        }

        let column = if entry.kind.is_pension() {
            age_group.pension_column()
        } else {
            age_group.salary_column()
        };
        if entry.kind == IncomeKind::OneTimeSalary {
            let percent = one_time_withholding_rate(column, totals.work_income);
            let withheld = percentage(gross, percent);
            return (
                withheld,
                withheld,
                0,
                AppliedWithholding::OneTimeTable(percent),
            );
        }

        let regular_gross = entry.annual_amount();
        let regular_withheld = if entry.kind.is_monthly() {
            (1..=12)
                .map(|month| table_withholding(table, column, entry.amount_for_month(month)))
                .fold(0_u32, u32::saturating_add)
        } else {
            annualized_table_withholding(table, column, regular_gross)
        };
        let vacation_gross = entry.vacation_compensation_amount();
        if vacation_gross == 0 {
            return (
                regular_withheld,
                regular_withheld,
                0,
                AppliedWithholding::Table(column),
            );
        }

        let percent = one_time_withholding_rate(column, totals.work_income);
        let supplemental_withheld = percentage(vacation_gross, percent);
        (
            regular_withheld.saturating_add(supplemental_withheld),
            regular_withheld,
            supplemental_withheld,
            AppliedWithholding::TableAndOneTime(column, percent),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncomePlanTotals {
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
}

impl IncomePlanTotals {
    pub const fn ordinary_income(self) -> u32 {
        self.work_income.saturating_add(self.pension_income)
    }

    /// Average monthly taxable salary and pension represented by this plan.
    pub const fn monthly_taxable_income(self) -> u32 {
        self.ordinary_income() / 12
    }

    pub const fn gross_income(self) -> u32 {
        self.ordinary_income().saturating_add(self.dividend_income)
    }

    pub const fn annual_profile(self) -> AnnualIncomeProfile {
        AnnualIncomeProfile {
            work_income: self.work_income,
            pension_income: self.pension_income,
        }
    }

    pub const fn total_employer_pension_contributions(self) -> u32 {
        self.regular_pension_premiums
            .saturating_add(self.vacation_pension_premiums)
            .saturating_add(self.salary_exchange_pension_contributions)
    }

    /// Total employer occupational-pension contributions as a percentage of
    /// the current-year pension-salary basis after salary exchange.
    pub fn employer_pension_share_of_basis(self) -> f64 {
        if self.pension_salary_basis == 0 {
            0.0
        } else {
            f64::from(self.total_employer_pension_contributions()) * 100.0
                / f64::from(self.pension_salary_basis)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SalaryExchangeAllowance {
    pub ceiling: u32,
    pub pension_salary_basis_before: u32,
    pub pension_salary_basis_after: u32,
    pub regular_pension_premiums: u32,
    pub vacation_pension_premiums: u32,
    pub other_exchange_contributions: u32,
    pub selected_exchange_contribution: u32,
    pub total_employer_pension_contributions: u32,
    pub available_contribution: u32,
    pub maximum_sacrifice: u32,
}

impl SalaryExchangeAllowance {
    pub fn contribution_share_of_basis(self) -> f64 {
        if self.pension_salary_basis_after == 0 {
            0.0
        } else {
            f64::from(self.total_employer_pension_contributions) * 100.0
                / f64::from(self.pension_salary_basis_after)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SalaryExchangeContext {
    regular_pension_premiums: u32,
    vacation_pension_premiums: u32,
    other_exchange_contributions: u32,
    other_pension_salary_basis: u32,
}

impl SalaryExchangeContext {
    pub fn allowance_for(
        self,
        payment_amount: u32,
        included_in_pension_salary_basis: bool,
        exchange: SalaryExchange,
    ) -> SalaryExchangeAllowance {
        let pension_contributions_before = self
            .regular_pension_premiums
            .saturating_add(self.vacation_pension_premiums)
            .saturating_add(self.other_exchange_contributions);
        let payment_in_basis = if included_in_pension_salary_basis {
            payment_amount
        } else {
            0
        };
        let pension_salary_basis_before = self
            .other_pension_salary_basis
            .saturating_add(payment_in_basis);
        let maximum_sacrifice = exchange.maximum_sacrifice(
            payment_amount,
            pension_salary_basis_before,
            pension_contributions_before,
            included_in_pension_salary_basis,
        );
        let sacrifice = exchange.sacrificed_salary.min(maximum_sacrifice);
        let pension_salary_basis_after = if included_in_pension_salary_basis {
            pension_salary_basis_before.saturating_sub(sacrifice)
        } else {
            pension_salary_basis_before
        };
        let ceiling = SalaryExchange::allowance_ceiling(pension_salary_basis_after);
        let mut applied_exchange = exchange;
        applied_exchange.sacrificed_salary = sacrifice;
        let selected_exchange_contribution = applied_exchange.pension_contribution();
        let total_employer_pension_contributions =
            pension_contributions_before.saturating_add(selected_exchange_contribution);

        SalaryExchangeAllowance {
            ceiling,
            pension_salary_basis_before,
            pension_salary_basis_after,
            regular_pension_premiums: self.regular_pension_premiums,
            vacation_pension_premiums: self.vacation_pension_premiums,
            other_exchange_contributions: self.other_exchange_contributions,
            selected_exchange_contribution,
            total_employer_pension_contributions,
            available_contribution: ceiling.saturating_sub(pension_contributions_before),
            maximum_sacrifice,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppliedWithholding {
    ActualAmount,
    Table(TaxColumn),
    TableAndOneTime(TaxColumn, u32),
    OneTimeTable(u32),
    Secondary30,
    AdjustmentPercent(u32),
    CustomPercent(u32),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntryWithholding {
    pub entry_id: u64,
    pub gross: u32,
    pub withheld: u32,
    pub regular_withheld: u32,
    pub supplemental_withheld: u32,
    pub rule: AppliedWithholding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithholdingSummary {
    pub total: u32,
    pub entries: Vec<EntryWithholding>,
}

fn table_withholding(table: u8, column: TaxColumn, income: u32) -> u32 {
    match monthly_deduction(table, column, income) {
        Some(TaxDeduction::Amount(amount)) => amount,
        Some(TaxDeduction::Percent(percent)) => percentage(income, percent),
        None => 0,
    }
}

fn annualized_table_withholding(table: u8, column: TaxColumn, annual_income: u32) -> u32 {
    let monthly_income = annual_income / 12;
    match monthly_deduction(table, column, monthly_income) {
        Some(TaxDeduction::Amount(amount)) => amount.saturating_mul(12),
        Some(TaxDeduction::Percent(percent)) => percentage(annual_income, percent),
        None => 0,
    }
}

fn percentage(amount: u32, percent: u32) -> u32 {
    (u64::from(amount) * u64::from(percent) / 100).min(u64::from(u32::MAX)) as u32
}

fn basis_points_rounded(amount: u32, basis_points: u32) -> u32 {
    ((u64::from(amount) * u64::from(basis_points) + 5_000) / 10_000).min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Calculation;

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
        assert_eq!(allowance.pension_salary_basis_before, 1_006_883);
        assert_eq!(allowance.pension_salary_basis_after, 1_006_883);
        assert_eq!(allowance.ceiling, 352_409);
        assert_eq!(allowance.regular_pension_premiums, 139_954);
        assert_eq!(allowance.vacation_pension_premiums, 34_765);
        assert_eq!(allowance.available_contribution, 177_690);
        assert_eq!(allowance.maximum_sacrifice, 168_012);

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
        assert_eq!(partial_totals.work_income, 1_278_883);

        plan.entries
            .iter_mut()
            .find(|entry| entry.id == lump_id)
            .unwrap()
            .salary_exchange
            .as_mut()
            .unwrap()
            .sacrificed_salary = allowance.maximum_sacrifice;

        let totals = plan.totals();
        assert_eq!(totals.work_income, 1_210_871);
        assert_eq!(totals.salary_exchange_sacrifice, 168_012);
        assert_eq!(totals.salary_exchange_pension_contributions, 177_689);
        assert_eq!(totals.total_employer_pension_contributions(), 352_408);
        assert_eq!(
            totals.employer_pension_share_of_basis(),
            f64::from(352_408) * 100.0 / f64::from(1_006_883)
        );
        let applied_allowance = plan.salary_exchange_allowance(lump_id).unwrap();
        assert_eq!(
            applied_allowance.total_employer_pension_contributions,
            352_408
        );
        assert_eq!(
            applied_allowance.contribution_share_of_basis(),
            f64::from(352_408) * 100.0 / f64::from(1_006_883)
        );
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
        assert_eq!(salary.vacation_compensation_amount(), 115_883);
        assert_eq!(salary.vacation_pension_premium_amount(), 34_765);

        let lump_id = plan.add_entry(IncomeKind::OneTimeSalary);
        let lump = plan
            .entries
            .iter_mut()
            .find(|entry| entry.id == lump_id)
            .unwrap();
        lump.amount = MONTHLY_SALARY * 4;
        lump.salary_exchange = Some(SalaryExchange::new());

        let allowance = plan.salary_exchange_allowance(lump_id).unwrap();
        assert_eq!(allowance.pension_salary_basis_before, 1_006_883);
        assert_eq!(allowance.ceiling, 352_409);
        assert_eq!(allowance.available_contribution, 177_690);
        assert_eq!(allowance.maximum_sacrifice, 168_012);

        let lump = plan
            .entries
            .iter_mut()
            .find(|entry| entry.id == lump_id)
            .unwrap();
        lump.salary_exchange.as_mut().unwrap().sacrificed_salary = allowance.maximum_sacrifice;
        assert_eq!(lump.total_annual_amount(), 203_988);
        assert_eq!(lump.salary_exchange_pension_contribution(), 177_689);

        let totals = plan.totals();
        assert_eq!(totals.work_income, 1_210_871);
        assert_eq!(totals.pension_salary_basis, 1_006_883);
        assert_eq!(totals.regular_pension_premiums, 139_954);
        assert_eq!(totals.vacation_pension_premiums, 34_765);
        assert_eq!(totals.salary_exchange_sacrifice, 168_012);
        assert_eq!(totals.salary_exchange_pension_contributions, 177_689);
        assert_eq!(totals.total_employer_pension_contributions(), 352_408);

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
    fn same_year_vacation_compensation_is_suggested_but_days_remain_editable() {
        let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
        entry.amount = 93_000;
        entry.start = Date2026::new(1, 1);
        entry.end = Date2026::new(10, 18);
        entry.vacation_compensation =
            Some(VacationCompensation::suggested(30, entry.start, entry.end));

        assert_eq!(entry.vacation_compensation.unwrap().payout_days, 24);
        assert_eq!(entry.vacation_compensation_amount(), 115_883);
        assert_eq!(entry.vacation_pension_premium_amount(), 34_765);
        assert_eq!(entry.pension_salary_basis_amount(), 1_006_883);
        assert_eq!(entry.total_annual_amount(), 1_006_883);

        entry.vacation_compensation.as_mut().unwrap().payout_days = 20;
        assert_eq!(entry.vacation_compensation_amount(), 96_569);
    }

    #[test]
    fn pensionability_and_actual_vacation_premium_are_editable() {
        let mut entry = IncomeEntry::new(1, IncomeKind::MonthlySalary);
        entry.amount = 93_000;
        entry.end = Date2026::new(10, 18);
        entry.vacation_compensation =
            Some(VacationCompensation::suggested(30, entry.start, entry.end));

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
        assert_eq!(totals.work_income, 1_006_883);
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
        plan.entries[0].custom_withholding_percent = Some(37);
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
    fn custom_withholding_editor_default_comes_from_the_income_entry() {
        let mut entry = IncomeEntry::new(1, IncomeKind::AnnualSalary);

        entry.set_custom_withholding_enabled(true);
        assert_eq!(entry.custom_withholding_percent, Some(30));

        entry.set_custom_withholding_enabled(false);
        assert_eq!(entry.custom_withholding_percent, None);
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
    fn default_and_adjusted_withholding_follow_precedence() {
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
            .custom_withholding_percent = Some(42);
        let summary = plan.estimated_withholding(32, TaxAgeGroup::Under66AtYearStart);
        let pension = summary
            .entries
            .iter()
            .find(|entry| entry.entry_id == secondary_id)
            .unwrap();
        assert_eq!(pension.withheld, 42_000);
        assert_eq!(pension.rule, AppliedWithholding::CustomPercent(42));
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
            entry.custom_withholding_percent = Some(42);
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
        }));
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
}
