//! Estimates how selected income uses the 2026 PGI and SGI ceilings.
//!
//! PGI figures follow Skatteverket and Pensionsmyndigheten. SGI figures follow
//! Försäkringskassan. The SGI result is only an estimate; Försäkringskassan
//! determines an individual's SGI when a qualifying benefit is claimed.

use crate::{TaxColumn, pension_fee, round_down_hundred};

/// Lowest 2026 annual income that can produce pensionable income (PGI).
pub const MINIMUM_PENSIONABLE_INCOME: u32 = 25_042;

/// Highest pensionable income (PGI) for income year 2026.
pub const MAXIMUM_PENSIONABLE_INCOME: u32 = 625_500;

/// 2026 income ceiling used when calculating the general pension fee.
pub const GENERAL_PENSION_FEE_INCOME_CEILING: u32 = 673_038;

/// Lowest 2026 annual work income that can produce an SGI.
pub const MINIMUM_SGI_INCOME: u32 = 14_200;

/// Highest sickness-benefit qualifying income (SGI) for 2026.
pub const MAXIMUM_SGI: u32 = 592_000;

/// An estimated annual income basis and its applicable 2026 ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncomeBasisProgress {
    pub estimated_basis: u32,
    pub maximum_basis: u32,
}

impl IncomeBasisProgress {
    const fn new(estimated_basis: u32, maximum_basis: u32) -> Self {
        Self {
            estimated_basis,
            maximum_basis,
        }
    }

    /// Percentage of the applicable annual maximum, capped at 100 percent.
    pub fn percent_of_maximum(self) -> f64 {
        f64::from(self.estimated_basis) * 100.0 / f64::from(self.maximum_basis)
    }
}

/// Whether the selected income is sufficient to estimate an income basis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncomeBasisEstimate {
    Estimated(IncomeBasisProgress),
    NotBasedOnSelectedIncome,
    RequiresAdditionalInformation,
}

/// Estimates 2026 pensionable income (PGI) from the selected annual income.
///
/// Salary and other compensation subject to the general pension fee can be
/// estimated directly. Pension payments do not produce new pension rights.
/// Sickness and activity compensation can include a pensionable amount based
/// on an assumed former income, which cannot be inferred from the payment.
pub fn public_pension_progress(column: TaxColumn, gross_yearly_income: u32) -> IncomeBasisEstimate {
    match column {
        TaxColumn::Column1 | TaxColumn::Column3 | TaxColumn::Column5 => {}
        TaxColumn::Column2 | TaxColumn::Column6 => {
            return IncomeBasisEstimate::NotBasedOnSelectedIncome;
        }
        TaxColumn::Column4 => {
            return IncomeBasisEstimate::RequiresAdditionalInformation;
        }
    }

    public_pension_progress_for_income(gross_yearly_income)
}

/// Estimates 2026 PGI from aggregate pensionable work income.
pub fn public_pension_progress_for_income(gross_yearly_income: u32) -> IncomeBasisEstimate {
    let assessed_income = round_down_hundred(gross_yearly_income);
    let pensionable_income = if assessed_income < MINIMUM_PENSIONABLE_INCOME {
        0
    } else {
        assessed_income
            .saturating_sub(pension_fee(assessed_income))
            .min(MAXIMUM_PENSIONABLE_INCOME)
    };

    IncomeBasisEstimate::Estimated(IncomeBasisProgress::new(
        pensionable_income,
        MAXIMUM_PENSIONABLE_INCOME,
    ))
}

/// Estimates 2026 sickness-benefit qualifying income (SGI).
///
/// The estimate applies only to salary columns and assumes recurring work.
/// Försäkringskassan determines the actual SGI when a benefit is claimed and
/// may take protected prior income and other work income into account.
pub fn estimated_sgi_progress(column: TaxColumn, gross_yearly_income: u32) -> IncomeBasisEstimate {
    if !matches!(column, TaxColumn::Column1 | TaxColumn::Column3) {
        return IncomeBasisEstimate::NotBasedOnSelectedIncome;
    }

    estimated_sgi_progress_for_income(gross_yearly_income)
}

/// Estimates 2026 SGI from an annualized recurring-work-income rate.
pub fn estimated_sgi_progress_for_income(gross_yearly_income: u32) -> IncomeBasisEstimate {
    let estimated_sgi = if gross_yearly_income < MINIMUM_SGI_INCOME {
        0
    } else {
        gross_yearly_income.min(MAXIMUM_SGI)
    };

    IncomeBasisEstimate::Estimated(IncomeBasisProgress::new(estimated_sgi, MAXIMUM_SGI))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_pension_progress_uses_pgi_after_the_general_pension_fee() {
        let estimate = public_pension_progress(TaxColumn::Column1, 510_000);

        assert_eq!(
            estimate,
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: 474_300,
                maximum_basis: MAXIMUM_PENSIONABLE_INCOME,
            })
        );
    }

    #[test]
    fn public_pension_progress_observes_the_minimum_and_maximum() {
        assert_eq!(
            public_pension_progress(TaxColumn::Column1, 25_000),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: 0,
                maximum_basis: MAXIMUM_PENSIONABLE_INCOME,
            })
        );
        assert_eq!(
            public_pension_progress(TaxColumn::Column1, 672_600),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: MAXIMUM_PENSIONABLE_INCOME,
                maximum_basis: MAXIMUM_PENSIONABLE_INCOME,
            })
        );
        assert_eq!(
            public_pension_progress(TaxColumn::Column5, 1_000_000),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: MAXIMUM_PENSIONABLE_INCOME,
                maximum_basis: MAXIMUM_PENSIONABLE_INCOME,
            })
        );
    }

    #[test]
    fn public_pension_progress_reports_columns_that_need_other_treatment() {
        for column in [TaxColumn::Column2, TaxColumn::Column6] {
            assert_eq!(
                public_pension_progress(column, 500_000),
                IncomeBasisEstimate::NotBasedOnSelectedIncome
            );
        }
        assert_eq!(
            public_pension_progress(TaxColumn::Column4, 500_000),
            IncomeBasisEstimate::RequiresAdditionalInformation
        );
    }

    #[test]
    fn estimated_sgi_uses_unrounded_annual_salary_and_caps_at_the_maximum() {
        assert_eq!(
            estimated_sgi_progress(TaxColumn::Column1, 14_199),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: 0,
                maximum_basis: MAXIMUM_SGI,
            })
        );
        assert_eq!(
            estimated_sgi_progress(TaxColumn::Column1, 14_201),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: 14_201,
                maximum_basis: MAXIMUM_SGI,
            })
        );
        assert_eq!(
            estimated_sgi_progress(TaxColumn::Column3, 700_000),
            IncomeBasisEstimate::Estimated(IncomeBasisProgress {
                estimated_basis: MAXIMUM_SGI,
                maximum_basis: MAXIMUM_SGI,
            })
        );
    }

    #[test]
    fn estimated_sgi_is_not_derived_from_non_salary_columns() {
        for column in [
            TaxColumn::Column2,
            TaxColumn::Column4,
            TaxColumn::Column5,
            TaxColumn::Column6,
        ] {
            assert_eq!(
                estimated_sgi_progress(column, 500_000),
                IncomeBasisEstimate::NotBasedOnSelectedIncome
            );
        }
    }

    #[test]
    fn percentage_uses_the_estimated_and_maximum_bases() {
        let progress = IncomeBasisProgress::new(296_000, MAXIMUM_SGI);

        assert_eq!(progress.percent_of_maximum(), 50.0);
    }
}
