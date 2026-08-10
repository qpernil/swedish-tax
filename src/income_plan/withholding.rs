use super::*;
use crate::{TaxAgeGroup, TaxColumn, TaxDeduction, monthly_deduction, one_time_withholding_rate};

impl IncomePlan {
    pub fn estimated_withholding(&self, table: u8, age_group: TaxAgeGroup) -> WithholdingSummary {
        let totals = self.totals();
        let mut entries = Vec::with_capacity(self.entries.len());
        let mut total = 0_u32;
        for entry in &self.entries {
            let gross = entry.total_annual_amount();
            let (withheld, regular_withheld, supplemental_withheld, additional_withheld, rule) =
                self.entry_withholding(entry, gross, totals, table, age_group);
            total = total.saturating_add(withheld);
            entries.push(EntryWithholding {
                entry_id: entry.id,
                gross,
                withheld,
                regular_withheld,
                supplemental_withheld,
                additional_withheld,
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
    ) -> (u32, u32, u32, u32, AppliedWithholding) {
        if let Some(withheld) = entry.actual_withholding {
            return (withheld, withheld, 0, 0, AppliedWithholding::ActualAmount);
        }
        if entry.kind.is_dividend() {
            return (0, 0, 0, 0, AppliedWithholding::None);
        }
        let (base, regular_withheld, supplemental_withheld, rule) =
            self.base_entry_withholding(entry, gross, totals, table, age_group);
        let additional_withheld = entry
            .requested_additional_withholding()
            .min(gross.saturating_sub(base));
        (
            base.saturating_add(additional_withheld),
            regular_withheld,
            supplemental_withheld,
            additional_withheld,
            rule,
        )
    }

    fn base_entry_withholding(
        &self,
        entry: &IncomeEntry,
        gross: u32,
        totals: IncomePlanTotals,
        table: u8,
        age_group: TaxAgeGroup,
    ) -> (u32, u32, u32, AppliedWithholding) {
        if entry.adjustment_applies
            && let Some(percent) = self.adjustment_percent
        {
            let withheld = percentage(gross, percent);
            return (
                withheld,
                withheld,
                0,
                AppliedWithholding::AdjustmentPercent(percent),
            );
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppliedWithholding {
    ActualAmount,
    Table(TaxColumn),
    TableAndOneTime(TaxColumn, u32),
    OneTimeTable(u32),
    Secondary30,
    AdjustmentPercent(u32),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntryWithholding {
    pub entry_id: u64,
    pub gross: u32,
    pub withheld: u32,
    pub regular_withheld: u32,
    pub supplemental_withheld: u32,
    pub additional_withheld: u32,
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
