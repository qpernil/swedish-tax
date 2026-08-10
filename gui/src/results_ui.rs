use super::*;

pub(super) fn summary_tiles(ui: &mut egui::Ui, summaries: &[Summary<'_>; 3]) {
    if ui.available_width() >= 720.0 {
        ui.columns(3, |columns| {
            for (column, summary) in columns.iter_mut().zip(summaries) {
                summary_tile(
                    column,
                    summary.label,
                    &summary.value,
                    summary.detail.as_deref(),
                    summary.value_color,
                    summary.detail_color,
                    summary.detail_help,
                );
            }
        });
    } else {
        for summary in summaries {
            summary_tile(
                ui,
                summary.label,
                &summary.value,
                summary.detail.as_deref(),
                summary.value_color,
                summary.detail_color,
                summary.detail_help,
            );
            ui.add_space(6.0);
        }
    }
}

pub(super) fn summary_tile(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    detail: Option<&str>,
    value_color: egui::Color32,
    detail_color: egui::Color32,
    detail_help: Option<HoverHelp>,
) {
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .inner_margin(15.0)
        .show(ui, |ui| {
            ui.set_min_height(86.0);
            ui.label(secondary_label(label));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(value)
                    .strong()
                    .size(20.0)
                    .color(value_color),
            );
            if let Some(detail) = detail {
                ui.add_space(4.0);
                let response = ui.label(
                    egui::RichText::new(detail)
                        .strong()
                        .size(13.0)
                        .color(detail_color),
                );
                if let Some(help) = detail_help {
                    response.on_hover_ui(help);
                }
            }
        });
}

pub(super) fn annual_reconciliation(ui: &mut egui::Ui, calculation: Calculation) {
    ui.label(
        egui::RichText::new("Annual reconciliation")
            .strong()
            .size(17.0)
            .color(primary_text()),
    );
    ui.add_space(8.0);
    let pension_component_count = [
        calculation.regular_pension_premiums,
        calculation.vacation_pension_premiums,
        calculation.salary_exchange_pension_contributions,
    ]
    .into_iter()
    .filter(|amount| *amount > 0)
    .count();
    egui::Grid::new("comparison-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(220.0)
        .show(ui, |ui| {
            value_row(
                ui,
                "Taxable salary and pension",
                format_sek(calculation.ordinary_income),
            );
            if calculation.dividend_income > 0 {
                value_row(
                    ui,
                    "Own-AB dividend",
                    format_sek(calculation.dividend_income),
                );
                value_row(
                    ui,
                    "Total income including dividend",
                    format_sek(calculation.annual_income),
                );
            }
            if pension_component_count > 1 {
                if calculation.regular_pension_premiums > 0 {
                    value_row(
                        ui,
                        "Regular pension contribution",
                        format_sek(calculation.regular_pension_premiums),
                    );
                }
                if calculation.vacation_pension_premiums > 0 {
                    value_row(
                        ui,
                        "Vacation-payout pension contribution",
                        format_sek(calculation.vacation_pension_premiums),
                    );
                }
                if calculation.salary_exchange_pension_contributions > 0 {
                    value_row(
                        ui,
                        "Salary-exchange pension contribution",
                        format_sek(calculation.salary_exchange_pension_contributions),
                    );
                }
            }
            if calculation.salary_exchange_sacrifice > 0 {
                value_row(
                    ui,
                    "Salary exchanged",
                    format_sek(calculation.salary_exchange_sacrifice),
                );
            }
            if calculation.employer_pension_contributions > 0 {
                value_row(
                    ui,
                    "Total tjänstepension contribution",
                    format!(
                        "{} · {:.2}% of pensionable salary after exchange",
                        format_sek(calculation.employer_pension_contributions),
                        calculation.employer_pension_share_of_basis(),
                    ),
                );
            }
            value_row(
                ui,
                "Effective final tax rate",
                format!("{:.2}%", calculation.effective_rate()),
            );
            value_row(
                ui,
                "Calculated tax withheld",
                format_sek(calculation.withheld_tax),
            );
            value_row(
                ui,
                "Cash after withholding",
                format_sek(calculation.cash_after_withholding()),
            );
            value_row(
                ui,
                if calculation.adjustment_calibration.is_some() {
                    "Jämkning-calibrated tax projection"
                } else {
                    "Estimated final tax"
                },
                format_sek(calculation.total_tax),
            );
            value_row(
                ui,
                "Expected balance",
                tax_balance_value(calculation.tax_balance_outcome()),
            );
        });
}

pub(super) fn calculation_trace(
    ui: &mut egui::Ui,
    plan: &IncomePlan,
    calculation: Calculation,
    table: u8,
    age_group: TaxAgeGroup,
) {
    let totals = plan.totals();
    let withholding = plan.estimated_withholding(table, age_group);
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(6.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Calculation trace")
                    .strong()
                    .size(17.0)
                    .color(primary_text()),
            );
            ui.label(
                egui::RichText::new(
                    "Follow each entered amount through cash income, payer withholding, annual tax, and the final balance.",
                )
                .small()
                .color(secondary_text()),
            );

            ui.add_space(10.0);
            trace_heading(ui, "1", "Cash income built from the entered rows");
            for (index, entry) in plan.entries.iter().enumerate() {
                trace_line(
                    ui,
                    &income_entry_name(entry, index),
                    cash_income_equation(entry),
                    format_sek(entry.total_annual_amount()),
                );
            }
            trace_line(
                ui,
                "Total cash income",
                format!(
                    "{} salary + {} pension + {} dividend",
                    format_sek(totals.work_income),
                    format_sek(totals.pension_income),
                    format_sek(totals.dividend_income),
                ),
                format_sek(totals.gross_income()),
            );

            ui.add_space(10.0);
            trace_heading(ui, "2", "Tax withheld by each payer");
            for (index, entry) in plan.entries.iter().enumerate() {
                let estimate = withholding
                    .entries
                    .iter()
                    .find(|estimate| estimate.entry_id == entry.id)
                    .copied();
                if let Some(estimate) = estimate {
                    trace_line(
                        ui,
                        &income_entry_name(entry, index),
                        withholding_equation(entry, estimate, table, totals.work_income),
                        format_sek(estimate.withheld),
                    );
                }
            }
            trace_line(
                ui,
                "Total tax withheld",
                "Sum of the payer amounts above",
                format_sek(withholding.total),
            );

            ui.add_space(10.0);
            trace_heading(ui, "3", "Annual tax projection");
            trace_line(
                ui,
                "Assessed income",
                format!(
                    "{} work income + {} pension income, rounded down to a whole hundred",
                    format_sek(totals.work_income),
                    format_sek(totals.pension_income),
                ),
                format_sek(calculation.annual_tax.assessed_income),
            );
            trace_line(
                ui,
                "Taxable income",
                format!(
                    "{} assessed income − {} basic allowance",
                    format_sek(calculation.annual_tax.assessed_income),
                    format_sek(calculation.annual_tax.basic_allowance),
                ),
                format_sek(calculation.annual_tax.taxable_income),
            );
            trace_line(
                ui,
                "Tax and fee additions",
                format!(
                    "{} state + {} municipal + {} burial/religious + {} pension fee + {} public service",
                    format_sek(calculation.annual_tax.state_income_tax),
                    format_sek(calculation.annual_tax.municipal_income_tax),
                    format_sek(calculation.annual_tax.burial_and_religious_fee),
                    format_sek(calculation.annual_tax.pension_fee),
                    format_sek(calculation.annual_tax.public_service_fee),
                ),
                format_sek(calculation.annual_tax.additions_total()),
            );
            trace_line(
                ui,
                "Tax credits",
                format!(
                    "{} pension fee + {} work income + {} sickness compensation + {} earned income",
                    format_sek(calculation.annual_tax.pension_fee_credit),
                    format_sek(calculation.annual_tax.work_income_credit),
                    format_sek(calculation.annual_tax.sickness_compensation_credit),
                    format_sek(calculation.annual_tax.earned_income_credit),
                ),
                format_credit(calculation.annual_tax.credits_total()),
            );
            trace_line(
                ui,
                "Annual formula tax",
                format!(
                    "{} additions − {} credits",
                    format_sek(calculation.annual_tax.additions_total()),
                    format_sek(calculation.annual_tax.credits_total()),
                ),
                format_sek(calculation.annual_tax.total),
            );
            if let Some(calibration) = calculation.adjustment_calibration {
                trace_line(
                    ui,
                    "Jämkning calibration",
                    format!(
                        "{} formula tax − {} implied adjustment from the {} basis",
                        format_sek(calculation.annual_tax.total),
                        format_signed_sek(calibration.implied_tax_adjustment),
                        format_sek(calibration.basis_income),
                    ),
                    format_sek(calibration.projected_ordinary_tax),
                );
            }
            if calculation.dividend_income > 0 {
                trace_line(
                    ui,
                    "Own-AB dividend tax",
                    format!(
                        "{} × {DIVIDEND_TAX_PERCENT}%",
                        format_sek(calculation.dividend_income)
                    ),
                    format_sek(calculation.dividend_tax),
                );
            }
            trace_line(
                ui,
                "Total final tax projection",
                if calculation.dividend_income > 0 {
                    "Projected salary/pension tax + dividend tax"
                } else {
                    "Projected salary and pension tax"
                },
                format_sek(calculation.total_tax),
            );

            ui.add_space(10.0);
            trace_heading(ui, "4", "Preliminary 2027 dividend allowance");
            if let Ok(allowance) = plan.dividend_allowance_2027() {
                trace_line(
                    ui,
                    "2027 basic amount",
                    format!(
                        "{} allocated by ownership and the multi-company cap",
                        format_sek(DIVIDEND_BASIC_AMOUNT_2027),
                    ),
                    format_sek(allowance.basic_amount),
                );
                trace_line(
                    ui,
                    "Marked 2026 owner salary",
                    "Cash salary, one-time salary, and vacation compensation from marked rows",
                    format_sek(allowance.owner_cash_salary),
                );
                trace_line(
                    ui,
                    "2027 wage-based allowance",
                    format!(
                        "50% of the ownership-adjusted 2026 payroll after {} deduction, limited to {}",
                        format_sek(DIVIDEND_WAGE_DEDUCTION_2027),
                        format_sek(allowance.wage_cap),
                    ),
                    format_sek(allowance.wage_allowance),
                );
                trace_line(
                    ui,
                    "Acquisition-cost interest",
                    format!(
                        "2027 rate applied to {} above the {} threshold",
                        format_sek(allowance.acquisition_cost_interest_basis),
                        format_sek(DIVIDEND_ACQUISITION_COST_THRESHOLD),
                    ),
                    format_sek(allowance.acquisition_cost_interest),
                );
                trace_line(
                    ui,
                    "2027 tax gränsbelopp",
                    format!(
                        "{} basic + {} wage + {} interest + {} saved allowance",
                        format_sek(allowance.basic_amount),
                        format_sek(allowance.wage_allowance),
                        format_sek(allowance.acquisition_cost_interest),
                        format_sek(allowance.saved_allowance),
                    ),
                    format_sek(allowance.total),
                );
                trace_line(
                    ui,
                    "Maximum dividend at 20%",
                    "Equals the tax gränsbelopp; company-law distribution capacity is separate",
                    format_sek(allowance.total),
                );
            } else {
                ui.colored_label(
                    egui::Color32::DARK_RED,
                    "Complete the 2027 dividend inputs to show this trace.",
                );
            }

            ui.add_space(10.0);
            trace_heading(ui, "5", "Reconciliation");
            if let Some(trace) = calculation.adjustment_balance_trace() {
                ui.label(
                    egui::RichText::new(
                        "The full-year jämkning projection is the zero-balance anchor. The expected balance shows how formula tax and withholding changed from that projection.",
                    )
                    .small()
                    .color(secondary_text()),
                );
                trace_line(
                    ui,
                    "Formula-tax change from projection",
                    "Actual-period formula tax − formula tax at the full-year basis",
                    format_delta_sek(trace.formula_tax_change),
                );
                trace_line(
                    ui,
                    "Withholding change from projection",
                    "Actual withholding − assumed withholding at the full-year basis",
                    format_delta_sek(trace.withholding_change),
                );
                trace_line(
                    ui,
                    "Formula change minus withholding change",
                    "Ordinary expected balance before dividend tax",
                    format_delta_sek(trace.ordinary_balance),
                );
            }
            let (equation, result) = match calculation.tax_balance_outcome() {
                TaxBalance::Debt(amount) => (
                    format!(
                        "{} final tax − {} withheld",
                        format_sek(calculation.total_tax),
                        format_sek(calculation.withheld_tax),
                    ),
                    tax_balance_value(TaxBalance::Debt(amount)),
                ),
                TaxBalance::Refund(amount) => (
                    format!(
                        "{} withheld − {} final tax",
                        format_sek(calculation.withheld_tax),
                        format_sek(calculation.total_tax),
                    ),
                    tax_balance_value(TaxBalance::Refund(amount)),
                ),
                TaxBalance::Settled => (
                    "Final tax equals tax withheld".to_owned(),
                    tax_balance_value(TaxBalance::Settled),
                ),
            };
            trace_line(ui, "Expected balance", equation, result);

            ui.add_space(10.0);
            trace_heading(ui, "5", "Income bases and tjänstepension");
            trace_line(
                ui,
                "Allmän pension (PGI)",
                format!(
                    "{} pensionable work income → general pension fee adjustment and 2026 cap",
                    format_sek(calculation.work_income),
                ),
                income_basis_trace_value(calculation.pension_progress),
            );
            trace_line(
                ui,
                "Estimated SGI",
                format!(
                    "{} annualized recurring salary rate → 2026 minimum and cap",
                    format_sek(calculation.sgi_annual_rate),
                ),
                income_basis_trace_value(calculation.sgi_progress),
            );
            trace_line(
                ui,
                "Total tjänstepension contributions",
                format!(
                    "{} regular + {} vacation payout + {} salary exchange",
                    format_sek(calculation.regular_pension_premiums),
                    format_sek(calculation.vacation_pension_premiums),
                    format_sek(calculation.salary_exchange_pension_contributions),
                ),
                format_sek(calculation.employer_pension_contributions),
            );
            trace_line(
                ui,
                "Share of pensionable salary after exchange",
                format!(
                    "{} total contributions ÷ {} pension-salary basis after exchange × 100",
                    format_sek(calculation.employer_pension_contributions),
                    format_sek(calculation.pension_salary_basis),
                ),
                format!("{:.2}%", calculation.employer_pension_share_of_basis()),
            );
        });
}

pub(super) fn trace_heading(ui: &mut egui::Ui, step: &str, title: &str) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(blue_color())
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(7, 2))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(step)
                        .strong()
                        .small()
                        .color(egui::Color32::WHITE),
                );
            });
        ui.label(egui::RichText::new(title).strong().color(primary_text()));
    });
}

pub(super) fn trace_line(
    ui: &mut egui::Ui,
    label: &str,
    equation: impl Into<String>,
    result: String,
) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(248, 250, 249))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(label).strong().color(primary_text()));
                ui.label(egui::RichText::new("=").color(secondary_text()));
                ui.label(egui::RichText::new(result).strong().color(blue_color()));
            });
            ui.label(
                egui::RichText::new(equation.into())
                    .small()
                    .color(secondary_text()),
            );
        });
}

pub(super) fn cash_income_equation(entry: &IncomeEntry) -> String {
    let base = entry.annual_amount();
    let vacation = entry.vacation_compensation_amount();
    let exchange = entry.salary_exchange_sacrifice();
    let base_text = if entry.kind.is_monthly() {
        format!(
            "{}/month over {} = {}",
            format_sek(entry.amount),
            entry_period_text(entry),
            format_sek(base),
        )
    } else {
        format!("Entered amount {}", format_sek(base))
    };
    match (vacation, exchange) {
        (0, 0) => base_text,
        (vacation, 0) => format!(
            "{base_text} + {} vacation compensation",
            format_sek(vacation)
        ),
        (0, exchange) => format!("{base_text} − {} salary exchange", format_sek(exchange)),
        (vacation, exchange) => format!(
            "{base_text} + {} vacation compensation − {} salary exchange",
            format_sek(vacation),
            format_sek(exchange),
        ),
    }
}

pub(super) fn withholding_equation(
    entry: &IncomeEntry,
    estimate: EntryWithholding,
    table: u8,
    annual_work_income: u32,
) -> String {
    let base = match estimate.rule {
        AppliedWithholding::ActualAmount => {
            "Actual tax withheld entered for this income row".to_owned()
        }
        AppliedWithholding::Table(column) if entry.kind.is_monthly() => format!(
            "Table {table}, column {}, summed for each paid month",
            column as u8
        ),
        AppliedWithholding::Table(column) => format!(
            "Table {table}, column {}, annual amount divided by 12 and applied for 12 months",
            column as u8
        ),
        AppliedWithholding::TableAndOneTime(column, percent) => {
            let vacation = entry.vacation_compensation_amount();
            format!(
                "{} regular table withholding (table {table}, column {}) + {} vacation × {percent}% (rate selected from {} annual work income)",
                format_sek(estimate.regular_withheld),
                column as u8,
                format_sek(vacation),
                format_sek(annual_work_income),
            )
        }
        AppliedWithholding::OneTimeTable(percent) => format!(
            "{} × one-time table {percent}% (rate selected from {} annual work income)",
            format_sek(estimate.gross),
            format_sek(annual_work_income),
        ),
        AppliedWithholding::Secondary30 => {
            format!(
                "{} × secondary-payer {SECONDARY_WITHHOLDING_PERCENT}%",
                format_sek(estimate.gross)
            )
        }
        AppliedWithholding::AdjustmentPercent(percent) => format!(
            "{} × percentage jämkning {percent}%",
            format_sek(estimate.gross)
        ),
        AppliedWithholding::None => "No preliminary withholding for this income type".to_owned(),
    };
    if estimate.additional_withheld > 0 {
        format!(
            "{base} + {} voluntary extra withholding",
            format_sek(estimate.additional_withheld),
        )
    } else {
        base
    }
}

pub(super) fn format_signed_sek(value: i64) -> String {
    if value < 0 {
        format!("−{}", format_sek(value.unsigned_abs() as u32))
    } else {
        format_sek(value as u32)
    }
}

pub(super) fn format_delta_sek(value: i64) -> String {
    if value > 0 {
        format!("+{}", format_sek(value as u32))
    } else {
        format_signed_sek(value)
    }
}

pub(super) fn income_basis_trace_value(estimate: IncomeBasisEstimate) -> String {
    match estimate {
        IncomeBasisEstimate::Estimated(progress) => format!(
            "{} of {}",
            format_sek(progress.estimated_basis),
            format_sek(progress.maximum_basis),
        ),
        IncomeBasisEstimate::NotBasedOnSelectedIncome => "Not based on selected income".to_owned(),
        IncomeBasisEstimate::RequiresAdditionalInformation => {
            "Requires additional information".to_owned()
        }
    }
}

pub(super) fn monthly_table_reference(
    ui: &mut egui::Ui,
    calculation: Calculation,
    table: u8,
    salary_column: TaxColumn,
) {
    ui.label(
        egui::RichText::new("Average monthly equivalent")
            .strong()
            .size(17.0)
            .color(primary_text()),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Reference only: annual taxable salary and pension divided by 12, then looked up as salary in the selected table. Actual payer withholding is calculated separately above.",
        )
        .small()
        .color(secondary_text()),
    );
    ui.add_space(8.0);
    egui::Grid::new("monthly-table-reference-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(220.0)
        .show(ui, |ui| {
            value_row(
                ui,
                "Average monthly taxable income",
                format_sek(calculation.monthly_income),
            );
            value_row(
                ui,
                &format!("Table {table}, salary column {}", salary_column as u8),
                table_deduction_text(calculation.table_deduction),
            );
            value_row(
                ui,
                "Annualized table deduction (12 months)",
                format_sek(calculation.annualized_table_reference_tax()),
            );
            value_row(
                ui,
                "Cash after table deduction",
                format_sek(calculation.table_reference_net()),
            );
        });
}

pub(super) fn income_basis_ceiling_progress(ui: &mut egui::Ui, calculation: Calculation) {
    let heading = ui.label(
        egui::RichText::new("Income-basis ceilings")
            .strong()
            .size(17.0)
            .color(primary_text()),
    );
    heading.on_hover_ui(income_basis_help);
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "PGI uses salary and one-time work compensation; SGI uses the annualized recurring salary rate.",
        )
            .small()
            .color(secondary_text()),
    );
    ui.add_space(8.0);

    egui::Grid::new("income-basis-ceilings-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(220.0)
        .show(ui, |ui| {
            income_basis_row(
                ui,
                "Allmän pension (PGI)",
                calculation.pension_progress,
                "Selected income does not earn new pension rights",
                "Requires the assumed income used for the compensation",
            );
            income_basis_row(
                ui,
                "Estimated SGI",
                calculation.sgi_progress,
                "SGI is not based on this selected income",
                "Requires additional income information",
            );
        });
}

pub(super) fn income_basis_row(
    ui: &mut egui::Ui,
    label: &str,
    estimate: IncomeBasisEstimate,
    not_based_text: &str,
    additional_information_text: &str,
) {
    ui.label(egui::RichText::new(label).color(secondary_text()));
    match estimate {
        IncomeBasisEstimate::Estimated(progress) => {
            ui.vertical(|ui| {
                let percent = progress.percent_of_maximum();
                ui.add(
                    egui::ProgressBar::new((percent / 100.0) as f32)
                        .desired_width(280.0)
                        .text(format!("{percent:.1}% of 2026 maximum")),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        format_sek(progress.estimated_basis),
                        format_sek(progress.maximum_basis),
                    ))
                    .small()
                    .color(secondary_text()),
                );
            });
        }
        IncomeBasisEstimate::NotBasedOnSelectedIncome => {
            ui.label(egui::RichText::new(not_based_text).color(secondary_text()));
        }
        IncomeBasisEstimate::RequiresAdditionalInformation => {
            ui.label(egui::RichText::new(additional_information_text).color(secondary_text()));
        }
    }
    ui.end_row();
}

pub(super) fn income_basis_help(ui: &mut egui::Ui) {
    ui.set_max_width(420.0);
    ui.label(egui::RichText::new("How the estimates are calculated").strong());
    ui.add_space(4.0);
    ui.label(
        "PGI is estimated after the general pension fee and compared with the 2026 maximum of \
         625,500 SEK.",
    );
    ui.add_space(4.0);
    ui.label(
        "SGI is estimated from recurring salary and compared with the 2026 maximum of 592,000 \
         SEK. Försäkringskassan determines the actual SGI when a benefit is claimed.",
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "SGI is the basis for sickness benefit and other social-insurance benefits; employer-paid sickness pay follows separate rules.",
        )
        .small()
        .color(secondary_text()),
    );
}

pub(super) fn annual_breakdown(ui: &mut egui::Ui, tax: AnnualTax) {
    egui::Grid::new("annual-breakdown-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(260.0)
        .show(ui, |ui| {
            value_row(ui, "Assessed income", format_sek(tax.assessed_income));
            value_row(ui, "Basic allowance", format_sek(tax.basic_allowance));
            value_row(ui, "Taxable income", format_sek(tax.taxable_income));
            value_row(ui, "State income tax", format_sek(tax.state_income_tax));
            value_row(
                ui,
                "Municipal income tax",
                format_sek(tax.municipal_income_tax),
            );
            value_row(
                ui,
                "Burial and religious fee",
                format_sek(tax.burial_and_religious_fee),
            );
            value_row(ui, "Pension fee", format_sek(tax.pension_fee));
            value_row(
                ui,
                "Pension fee credit",
                format_credit(tax.pension_fee_credit),
            );
            value_row(
                ui,
                "Work income credit",
                format_credit(tax.work_income_credit),
            );
            value_row(
                ui,
                "Sickness compensation credit",
                format_credit(tax.sickness_compensation_credit),
            );
            value_row(
                ui,
                "Earned income credit",
                format_credit(tax.earned_income_credit),
            );
            value_row(ui, "Public service fee", format_sek(tax.public_service_fee));
            value_row(ui, "Total annual tax", format_sek(tax.total));
        });
}

pub(super) fn adjustment_calibration_breakdown(
    ui: &mut egui::Ui,
    calibration: AdjustmentCalibration,
) {
    ui.label(
        egui::RichText::new(
            "Jämkning calibration assumes the full-year basis would have produced zero balance at the entered percentage. The implied adjustment is then held constant for the actual annual income.",
        )
        .small()
        .color(secondary_text()),
    );
    ui.add_space(4.0);
    egui::Grid::new("adjustment-calibration-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(260.0)
        .show(ui, |ui| {
            value_row(
                ui,
                "Full-year jämkning basis",
                format_sek(calibration.basis_income),
            );
            value_row(
                ui,
                &format!("Assumed zero-balance tax at {}%", calibration.percent),
                format_sek(calibration.assumed_tax_at_basis),
            );
            value_row(
                ui,
                "Formula tax at the basis income",
                format_sek(calibration.formula_tax_at_basis),
            );
            let adjustment = match calibration.implied_tax_adjustment.cmp(&0) {
                std::cmp::Ordering::Greater => {
                    format!("−{}", format_sek(calibration.implied_tax_adjustment as u32))
                }
                std::cmp::Ordering::Less => format!(
                    "+{}",
                    format_sek(calibration.implied_tax_adjustment.unsigned_abs() as u32)
                ),
                std::cmp::Ordering::Equal => format_sek(0),
            };
            value_row(ui, "Implied adjustment to formula tax", adjustment);
            value_row(
                ui,
                "Jämkning-calibrated ordinary tax projection",
                format_sek(calibration.projected_ordinary_tax),
            );
        });
}

pub(super) fn value_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(egui::RichText::new(label).color(secondary_text()));
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new(value).strong().color(primary_text()));
    });
    ui.end_row();
}

pub(super) fn table_deduction_text(deduction: TaxDeduction) -> String {
    match deduction {
        TaxDeduction::Amount(amount) => format!("{} / month", format_sek(amount)),
        TaxDeduction::Percent(percent) => format!("{percent}% of payment"),
    }
}

pub(super) fn tax_balance_summary(balance: TaxBalance) -> String {
    match balance {
        TaxBalance::Debt(_) => format!(
            "Expected balance: {} · Tax debt",
            tax_balance_value(balance)
        ),
        TaxBalance::Refund(_) => {
            format!(
                "Expected balance: {} · Tax refund",
                tax_balance_value(balance)
            )
        }
        TaxBalance::Settled => "No expected balance · Settled".to_owned(),
    }
}

pub(super) fn tax_balance_value(balance: TaxBalance) -> String {
    match balance {
        TaxBalance::Debt(amount) => format!("−{}", format_sek(amount)),
        TaxBalance::Refund(amount) => format!("+{}", format_sek(amount)),
        TaxBalance::Settled => format_sek(0),
    }
}

pub(super) fn tax_balance_color(balance: TaxBalance) -> egui::Color32 {
    match balance {
        TaxBalance::Debt(_) => egui::Color32::from_rgb(176, 42, 42),
        TaxBalance::Refund(_) => green_color(),
        TaxBalance::Settled => primary_text(),
    }
}

pub(super) fn format_credit(value: u32) -> String {
    if value == 0 {
        format_sek(0)
    } else {
        format!("-{}", format_sek(value))
    }
}

pub(super) fn format_basis_points_percentage(basis_points: u32) -> String {
    let whole_percent = basis_points / 100;
    let fractional_percent = basis_points % 100;
    match fractional_percent {
        0 => whole_percent.to_string(),
        fraction if fraction.is_multiple_of(10) => {
            format!("{whole_percent}.{}", fraction / 10)
        }
        fraction => format!("{whole_percent}.{fraction:02}"),
    }
}

pub(super) fn parse_basis_points_percentage(text: &str) -> Option<u32> {
    let normalized = text.replace(',', ".");
    let mut parts = normalized.split('.');
    let whole_text = parts.next().unwrap_or_default();
    let fractional_text = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || fractional_text.len() > 2
        || !whole_text
            .chars()
            .all(|character| character.is_ascii_digit())
        || !fractional_text
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return None;
    }

    let whole_percent = if whole_text.is_empty() {
        0
    } else {
        whole_text.parse::<u64>().ok()?
    };
    let fractional_basis_points = match fractional_text.len() {
        0 => 0,
        1 => fractional_text.parse::<u64>().ok()? * 10,
        2 => fractional_text.parse::<u64>().ok()?,
        _ => return None,
    };
    let total = whole_percent
        .checked_mul(100)?
        .checked_add(fractional_basis_points)?;
    u32::try_from(total).ok()
}

pub(super) fn sanitize_percentage_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut has_separator = false;
    let mut fractional_digits = 0;
    for character in input.chars() {
        if character.is_ascii_digit() {
            if !has_separator || fractional_digits < 2 {
                output.push(character);
                if has_separator {
                    fractional_digits += 1;
                }
            }
        } else if matches!(character, '.' | ',') && !has_separator {
            output.push('.');
            has_separator = true;
        }
    }
    output
}

pub(super) fn format_sek(value: u32) -> String {
    format!("{} SEK", grouped_digits(value))
}

pub(super) fn grouped_digits(value: u32) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(character);
    }
    grouped
}

pub(super) fn secondary_label(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .small()
        .strong()
        .color(secondary_text())
}

pub(super) fn background_color() -> egui::Color32 {
    egui::Color32::from_rgb(244, 247, 246)
}

pub(super) fn surface_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 255, 255)
}

pub(super) fn border_color() -> egui::Color32 {
    egui::Color32::from_rgb(210, 218, 215)
}

pub(super) fn primary_text() -> egui::Color32 {
    egui::Color32::from_rgb(30, 44, 41)
}

pub(super) fn secondary_text() -> egui::Color32 {
    egui::Color32::from_rgb(91, 105, 101)
}

pub(super) fn blue_color() -> egui::Color32 {
    egui::Color32::from_rgb(0, 82, 147)
}

pub(super) fn green_color() -> egui::Color32 {
    egui::Color32::from_rgb(24, 121, 78)
}

pub(super) fn yellow_text() -> egui::Color32 {
    egui::Color32::from_rgb(128, 91, 0)
}
