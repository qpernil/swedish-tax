use super::*;

pub(super) fn income_overview_table(
    ui: &mut egui::Ui,
    plan: &IncomePlan,
    withholding: &WithholdingSummary,
    selected_id: &mut Option<u64>,
) {
    let totals = plan.totals();
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Income rows")
                    .strong()
                    .size(16.0)
                    .color(primary_text()),
            );
            ui.label(
                egui::RichText::new(
                    "Select a row to edit it. Every amount below updates immediately.",
                )
                .small()
                .color(secondary_text()),
            );
            ui.add_space(6.0);

            egui::Grid::new("income-entry-overview")
                .num_columns(6)
                .striped(true)
                .min_col_width(105.0)
                .show(ui, |ui| {
                    ui.label(secondary_label("Income"));
                    ui.label(secondary_label("Cash income"));
                    ui.label(secondary_label("Withheld"));
                    ui.label(secondary_label("Jämkning basis"));
                    ui.label(secondary_label("Employer pension"));
                    ui.label(secondary_label("PGI / SGI"));
                    ui.end_row();

                    for (index, entry) in plan.entries.iter().enumerate() {
                        let response = ui
                            .vertical(|ui| {
                                let response = ui.selectable_label(
                                    *selected_id == Some(entry.id),
                                    income_entry_name(entry, index),
                                );
                                ui.label(
                                    egui::RichText::new(income_kind_short(entry.kind))
                                        .small()
                                        .color(secondary_text()),
                                );
                                if entry.kind.is_monthly() {
                                    ui.label(
                                        egui::RichText::new(entry_period_text(entry))
                                            .small()
                                            .color(secondary_text()),
                                    );
                                }
                                response
                            })
                            .inner;
                        if response.clicked() {
                            *selected_id = Some(entry.id);
                        }

                        ui.label(format_sek(entry.total_annual_amount()));
                        let withheld = withholding
                            .entries
                            .iter()
                            .find(|estimate| estimate.entry_id == entry.id)
                            .map(|estimate| estimate.withheld)
                            .unwrap_or(0);
                        ui.label(format_sek(withheld));
                        ui.label(optional_sek(entry.full_year_adjustment_basis_amount()));
                        let employer_pension = entry.total_employer_pension_contribution();
                        ui.label(optional_sek(employer_pension));
                        ui.label(income_eligibility_short(entry.kind));
                        ui.end_row();
                    }

                    ui.label(egui::RichText::new("Total").strong());
                    ui.label(egui::RichText::new(format_sek(totals.gross_income())).strong());
                    ui.label(egui::RichText::new(format_sek(withholding.total)).strong());
                    ui.label(
                        egui::RichText::new(optional_sek(totals.adjustment_basis_work_income))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(optional_sek(
                            totals.total_employer_pension_contributions(),
                        ))
                        .strong(),
                    );
                    ui.label(egui::RichText::new("—").strong());
                    ui.end_row();
                });
        });
}

pub(super) fn income_entry_name(entry: &IncomeEntry, index: usize) -> String {
    if entry.description.trim().is_empty() {
        format!("Income {}", index + 1)
    } else {
        entry.description.trim().to_owned()
    }
}

pub(super) fn entry_period_text(entry: &IncomeEntry) -> String {
    format!(
        "{} {} – {} {}",
        month_name(entry.start.month),
        entry.start.day,
        month_name(entry.end.month),
        entry.end.day,
    )
}

pub(super) fn optional_sek(value: u32) -> String {
    if value == 0 {
        "—".to_owned()
    } else {
        format_sek(value)
    }
}

pub(super) fn income_eligibility_short(kind: IncomeKind) -> &'static str {
    match (kind.is_pgi_eligible(), kind.is_sgi_eligible()) {
        (true, true) => "PGI + SGI",
        (true, false) => "PGI",
        _ => "—",
    }
}

pub(super) fn income_kind_short(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::AnnualSalary => "Annual salary",
        IncomeKind::MonthlySalary => "Monthly salary",
        IncomeKind::OneTimeSalary => "One-time salary",
        IncomeKind::MonthlyOccupationalPension => "Monthly tjänstepension",
        IncomeKind::AnnualOccupationalPension => "Annual tjänstepension",
        IncomeKind::OwnCompanyDividend => "Own-AB dividend",
    }
}

pub(super) fn selected_income_impact(
    ui: &mut egui::Ui,
    plan: &IncomePlan,
    selected_id: Option<u64>,
    table: u8,
    age_group: TaxAgeGroup,
) {
    let Some(entry) = plan
        .entries
        .iter()
        .find(|entry| Some(entry.id) == selected_id)
    else {
        ui.label(
            egui::RichText::new("Select an income row to inspect its calculation.")
                .color(secondary_text()),
        );
        return;
    };
    let withholding = plan
        .estimated_withholding(table, age_group)
        .entries
        .into_iter()
        .find(|estimate| estimate.entry_id == entry.id);

    audit_section(ui, "Cash income", |ui| {
        audit_row(
            ui,
            "Base amount for the period",
            format_sek(entry.annual_amount()),
        );
        if entry.vacation_compensation_amount() > 0 {
            audit_row(
                ui,
                "Vacation compensation added",
                format!("+{}", format_sek(entry.vacation_compensation_amount())),
            );
        }
        if entry.salary_exchange_sacrifice() > 0 {
            audit_row(
                ui,
                "Salary exchanged",
                format!("−{}", format_sek(entry.salary_exchange_sacrifice())),
            );
        }
        audit_row(
            ui,
            "Cash used by tax calculation",
            format_sek(entry.total_annual_amount()),
        );
    });

    ui.add_space(6.0);
    audit_section(ui, "Tax and withholding", |ui| {
        if entry.kind.is_dividend() {
            audit_row(
                ui,
                "Final-tax category",
                format!("Own-AB dividend at {DIVIDEND_TAX_PERCENT}%"),
            );
            audit_row(ui, "Preliminary withholding", "None");
        } else if let Some(withholding) = withholding {
            audit_row(
                ui,
                "Tax category",
                match entry.kind.tax_category() {
                    IncomeTaxCategory::Pension => "Pension income",
                    IncomeTaxCategory::Work => "Work income",
                    IncomeTaxCategory::Dividend => "Dividend income",
                },
            );
            audit_row(
                ui,
                "Withholding rule",
                withholding_rule_text(withholding.rule),
            );
            audit_row(ui, "Tax withheld", format_sek(withholding.withheld));
        }
        if let Some(percent) = plan.adjustment_percent {
            audit_row(
                ui,
                "Uses jämkning",
                if entry.adjustment_applies {
                    format!("Yes — {percent}% withholding")
                } else {
                    "No — payer uses its other rule".to_owned()
                },
            );
            audit_row(
                ui,
                "Used as full-year jämkning basis",
                if entry.full_year_adjustment_basis_amount() > 0 {
                    format_sek(entry.full_year_adjustment_basis_amount())
                } else {
                    "No".to_owned()
                },
            );
        }
    });

    ui.add_space(6.0);
    audit_section(ui, "Pension", |ui| {
        audit_row(
            ui,
            "Current-year pensionable salary",
            optional_sek(entry.pension_salary_basis_amount()),
        );
        audit_row(
            ui,
            "Regular employer contribution",
            optional_sek(entry.regular_pension_premium_amount()),
        );
        if entry.vacation_compensation_amount() > 0 {
            let vacation_in_basis = entry
                .vacation_compensation
                .is_some_and(|vacation| vacation.included_in_pension_salary_basis);
            audit_row(
                ui,
                "Vacation payout in pension basis",
                if vacation_in_basis { "Yes" } else { "No" },
            );
            audit_row(
                ui,
                "Vacation employer contribution",
                optional_sek(entry.vacation_pension_premium_amount()),
            );
        }
        if entry.salary_exchange.is_some() {
            audit_row(
                ui,
                "Salary-exchange contribution",
                optional_sek(entry.salary_exchange_pension_contribution()),
            );
        }
    });

    ui.add_space(6.0);
    audit_section(ui, "PGI and SGI", |ui| {
        audit_row(
            ui,
            "PGI-eligible income",
            entry
                .pgi_eligible_income()
                .map(format_sek)
                .unwrap_or_else(|| "Not included".to_owned()),
        );
        audit_row(
            ui,
            "SGI annual-rate estimate",
            entry
                .sgi_annual_rate()
                .map(format_sek)
                .unwrap_or_else(|| "Not included".to_owned()),
        );
    });
}

pub(super) fn audit_section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(248, 250, 249))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(title).strong().color(primary_text()));
            ui.add_space(3.0);
            contents(ui);
        });
}

pub(super) fn audit_row(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("✓").strong().color(green_color()));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).small().color(secondary_text()));
            ui.label(
                egui::RichText::new(value.into())
                    .small()
                    .strong()
                    .color(primary_text()),
            );
        });
    });
}

pub(super) fn income_totals_footer(
    ui: &mut egui::Ui,
    plan: &IncomePlan,
    table: u8,
    age_group: TaxAgeGroup,
) {
    let totals = plan.totals();
    let withheld = plan.estimated_withholding(table, age_group).total;
    let calculation = Calculation::new(table, age_group, plan);
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(242, 247, 245))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Year totals")
                    .strong()
                    .color(primary_text()),
            );
            ui.add_space(5.0);
            ui.columns(4, |columns| {
                compact_fact(
                    &mut columns[0],
                    "Total cash income",
                    format_sek(totals.gross_income()),
                );
                compact_fact(
                    &mut columns[1],
                    "Tax withheld",
                    format_sek(withheld),
                );
                compact_fact(
                    &mut columns[2],
                    "Final tax projection",
                    calculation
                        .map(|calculation| format_sek(calculation.total_tax))
                        .unwrap_or_else(|| "Invalid period".to_owned()),
                );
                compact_fact(
                    &mut columns[3],
                    "Projected balance",
                    calculation
                        .map(|calculation| {
                            tax_balance_summary(calculation.tax_balance_outcome())
                        })
                        .unwrap_or_else(|| "—".to_owned()),
                );
            });
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new(format!(
                    "Salary {} · Tjänstepension received {} · Dividend {} · Modeled employer pension contributions {}",
                    format_sek(totals.work_income),
                    format_sek(totals.pension_income),
                    format_sek(totals.dividend_income),
                    format_sek(totals.total_employer_pension_contributions()),
                ))
                .small()
                .color(secondary_text()),
            );
        });
}

pub(super) fn compact_fact(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.label(secondary_label(label));
        ui.label(egui::RichText::new(value).strong().color(primary_text()));
    });
}

pub(super) fn card(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(8.0)
        .inner_margin(16.0)
        .show(ui, contents);
}

pub(super) fn card_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .strong()
            .size(16.0)
            .color(primary_text()),
    );
    ui.label(
        egui::RichText::new(subtitle)
            .small()
            .color(secondary_text()),
    );
}

pub(super) fn income_entry_editor(
    ui: &mut egui::Ui,
    entry: &mut IncomeEntry,
    adjustment: Option<u32>,
    withholding: Option<EntryWithholding>,
    pension_context: SalaryExchangeContext,
    focus_description: bool,
) {
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.label(secondary_label("Description"));
                    let description_response = ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::TextEdit::singleline(&mut entry.description)
                            .hint_text("Employer or payment"),
                    );
                    if focus_description {
                        description_response.request_focus();
                    }
                });
                columns[1].vertical(|ui| {
                    ui.label(secondary_label(if entry.kind.is_monthly() {
                        "Amount per month"
                    } else {
                        "Annual / one-time amount"
                    }));
                    ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::DragValue::new(&mut entry.amount)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(1_000.0),
                    );
                });
            });
            ui.add_space(6.0);
            ui.label(secondary_label("Income type"));
            egui::ComboBox::from_id_salt(("income-kind", entry.id))
                .selected_text(entry.kind.label())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    let mut selected_kind = entry.kind;
                    for kind in IncomeKind::ALL {
                        ui.selectable_value(&mut selected_kind, kind, kind.label());
                    }
                    entry.set_kind(selected_kind, adjustment.is_some());
                });

            if entry.kind.is_monthly() {
                ui.add_space(8.0);
                ui.label(secondary_label("Payment period"));
                let mut period_changed = false;
                ui.columns(2, |columns| {
                    period_changed |=
                        date_editor(&mut columns[0], "First day", entry.id, &mut entry.start);
                    period_changed |= date_editor(
                        &mut columns[1],
                        "Last day",
                        entry.id + 1_000_000,
                        &mut entry.end,
                    );
                });
                ui.label(
                    egui::RichText::new("Both the first and last day are included.")
                        .small()
                        .color(secondary_text()),
                );
                ui.checkbox(
                    &mut entry.use_annual_daily_rate_for_partial_months,
                    "Use annual daily rate for partial months (12/365)",
                );
                ui.label(
                    egui::RichText::new(if entry.use_annual_daily_rate_for_partial_months {
                        "Partial months use monthly amount × 12 ÷ 365."
                    } else {
                        "Partial months use the number of calendar days in that month (default)."
                    })
                    .small()
                    .color(secondary_text()),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Full year").clicked() {
                        entry.start = Date2026::new(1, 1);
                        entry.end = Date2026::new(12, 31);
                        period_changed = true;
                    }
                    if !entry.is_valid() {
                        ui.colored_label(
                            egui::Color32::DARK_RED,
                            "Last day must be on or after first day",
                        );
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "Annual amount: {}",
                            format_sek(entry.annual_amount())
                        ))
                        .small()
                        .strong(),
                    );
                });
                if entry.kind == IncomeKind::MonthlySalary {
                    ui.add_space(8.0);
                    vacation_compensation_editor(ui, entry, period_changed);
                }
            }

            if entry.kind.is_salary() {
                ui.add_space(8.0);
                ui.checkbox(
                    &mut entry.own_company_sourced,
                    "Paid by my own company or qualifying group",
                );
                ui.label(
                    egui::RichText::new(
                        "This 2026 cash compensation feeds the preliminary 2027 3:12 owner-salary and payroll calculation.",
                    )
                    .small()
                    .color(secondary_text()),
                );
            }

            if matches!(
                entry.kind,
                IncomeKind::AnnualSalary | IncomeKind::MonthlySalary
            ) {
                if let Some(percent) = adjustment {
                    ui.add_space(8.0);
                    adjustment_basis_editor(ui, entry, percent);
                }
                ui.add_space(8.0);
                regular_pension_premium_editor(ui, entry);
            }
            if entry.kind == IncomeKind::OneTimeSalary {
                ui.add_space(8.0);
                salary_exchange_editor(ui, entry, pension_context);
            }

            ui.add_space(8.0);
            if entry.kind.is_dividend() {
                ui.label(eligibility_badge(&format!(
                    "{DIVIDEND_TAX_PERCENT}% final tax within gränsbelopp · 0 SEK default withholding · no PGI · no SGI"
                )));
            } else {
                ui.label(secondary_label("Payer"));
                let mut payer_role = entry.payer_role;
                egui::ComboBox::from_id_salt(("payer-role", entry.id))
                    .selected_text(match payer_role {
                        PayerRole::Main => "Main payer",
                        PayerRole::Secondary => "Secondary payer",
                    })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut payer_role,
                            PayerRole::Main,
                            "Main payer — table",
                        );
                        ui.selectable_value(
                            &mut payer_role,
                            PayerRole::Secondary,
                            format!("Secondary payer — {SECONDARY_WITHHOLDING_PERCENT}%"),
                        );
                    });
                entry.set_payer_role(payer_role, adjustment.is_some());
                if adjustment.is_some() {
                    ui.checkbox(
                        &mut entry.adjustment_applies,
                        "Use jämkning",
                    );
                } else {
                    entry.adjustment_applies = false;
                }

                ui.label(eligibility_badge(income_eligibility(entry.kind)));
            }

            ui.add_space(8.0);
            let mut use_actual = entry.actual_withholding.is_some();
            if ui
                .checkbox(&mut use_actual, "Use actual tax withheld")
                .changed()
            {
                entry.set_actual_withholding_enabled(use_actual);
                if use_actual {
                    entry.additional_withholding_per_payment = None;
                }
            }
            if let Some(actual) = &mut entry.actual_withholding {
                ui.horizontal_wrapped(|ui| {
                    ui.label(secondary_label("Actual tax withheld"));
                    ui.add(
                        egui::DragValue::new(actual)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(100.0),
                    );
                });
                ui.label(
                    egui::RichText::new(
                        "Use this after payments have been made. It replaces the estimated table, secondary-payer, jämkning, and voluntary-extra amounts for this row.",
                    )
                    .small()
                    .color(secondary_text()),
                );
            }
            if !entry.kind.is_dividend() {
                let mut use_additional = entry.additional_withholding_per_payment.is_some();
                if ui
                    .checkbox(&mut use_additional, "Add voluntary extra withholding")
                    .changed()
                {
                    entry.set_additional_withholding_enabled(use_additional);
                    if use_additional {
                        entry.actual_withholding = None;
                    }
                }
                let payment_count = entry.withholding_payment_count();
                if let Some(additional) = &mut entry.additional_withholding_per_payment {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(secondary_label(if payment_count > 1 {
                            "Extra per payment"
                        } else {
                            "Extra withholding"
                        }));
                        ui.add(
                            egui::DragValue::new(additional)
                                .range(0..=MAX_INCOME)
                                .suffix(" SEK")
                                .speed(100.0),
                        );
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "Added on top of the normal withholding for each payment. Planned extra for period: {}.",
                            format_sek(additional.saturating_mul(payment_count)),
                        ))
                        .small()
                        .color(secondary_text()),
                    );
                }
            }
            if let Some(withholding) = withholding {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Withholding used: {} · {}",
                        format_sek(withholding.withheld),
                        if withholding.additional_withheld > 0 {
                            format!(
                                "{} + {} voluntary extra",
                                withholding_rule_text(withholding.rule),
                                format_sek(withholding.additional_withheld),
                            )
                        } else {
                            withholding_rule_text(withholding.rule)
                        },
                    ))
                    .small()
                    .color(secondary_text()),
                );
            }
        });
}

pub(super) fn adjustment_basis_editor(ui: &mut egui::Ui, entry: &mut IncomeEntry, percent: u32) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(252, 249, 239))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(222, 207, 163),
        ))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.checkbox(
                &mut entry.use_full_year_projection_as_adjustment_basis,
                "Use full-year projection as jämkning basis",
            );
            ui.label(
                egui::RichText::new(
                    "Projects this recurring income over all 12 months and assumes the entered percentage jämkning was calculated from that annual amount. This does not change the actual income period.",
                )
                .small()
                .color(secondary_text()),
            );
            if entry.use_full_year_projection_as_adjustment_basis {
                let basis = entry.full_year_adjustment_basis_amount();
                ui.add_space(4.0);
                let basis_text = if entry.kind == IncomeKind::MonthlySalary {
                    format!(
                        "Jämkning basis: {} × 12 = {} at {percent}%",
                        format_sek(entry.amount),
                        format_sek(basis),
                    )
                } else {
                    format!("Jämkning basis: {} at {percent}%", format_sek(basis))
                };
                ui.label(
                    egui::RichText::new(basis_text)
                        .strong()
                        .color(primary_text()),
                );
            }
        });
}

pub(super) fn regular_pension_premium_editor(ui: &mut egui::Ui, entry: &mut IncomeEntry) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(242, 247, 252))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(190, 210, 225),
        ))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.checkbox(
                &mut entry.included_in_pension_salary_basis,
                "Treat this salary as pensionable",
            );
            let mut enabled = entry.regular_pension_premium.is_some();
            if ui
                .checkbox(&mut enabled, "Calculate employer tjänstepension")
                .changed()
            {
                entry.regular_pension_premium = enabled.then_some(RegularPensionPremium::default());
            }
            let Some(mut premium) = entry.regular_pension_premium else {
                ui.label(
                    egui::RichText::new(
                        "No employer pension contribution included for this salary.",
                    )
                    .small()
                    .color(secondary_text()),
                );
                return;
            };

            let benchmark = entry.regular_pension_benchmark_monthly().unwrap_or(0);
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new(
                    "2026 individual-pension benchmark: 4.5% through 52 125 SEK/month, then 30%.",
                )
                .small()
                .color(secondary_text()),
            );
            ui.label(format!("Benchmark: {}", format_sek(benchmark)));
            let mut use_override = premium.monthly_override.is_some();
            if ui
                .checkbox(&mut use_override, "Use actual monthly pension contribution")
                .changed()
            {
                premium.monthly_override = use_override.then_some(benchmark);
            }
            if let Some(actual) = &mut premium.monthly_override {
                ui.label(secondary_label("Actual monthly pension contribution"));
                ui.add_sized(
                    [ui.available_width().min(180.0), 28.0],
                    egui::DragValue::new(actual)
                        .range(0..=MAX_INCOME)
                        .suffix(" SEK")
                        .speed(100.0),
                );
            }
            entry.regular_pension_premium = Some(premium);
            ui.label(
                egui::RichText::new(format!(
                    "Estimated contribution for this salary period: {}",
                    format_sek(entry.regular_pension_premium_amount()),
                ))
                .strong()
                .color(blue_color()),
            );
        });
}

pub(super) fn salary_exchange_editor(
    ui: &mut egui::Ui,
    entry: &mut IncomeEntry,
    context: SalaryExchangeContext,
) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(244, 250, 246))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(184, 214, 197),
        ))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.checkbox(
                &mut entry.included_in_pension_salary_basis,
                "Treat this payment as pensionable",
            )
            .on_hover_text(
                "Off by default for a termination payment. Turn it on if the employment agreement treats this cash payment as pensionable salary.",
            );
            let mut enabled = entry.salary_exchange.is_some();
            if ui
                .checkbox(
                    &mut enabled,
                    "Exchange part of this payment for tjänstepension",
                )
                .changed()
            {
                if enabled {
                    let mut exchange = SalaryExchange::new();
                    exchange.sacrificed_salary = context
                        .allowance_for(
                            entry.amount,
                            entry.included_in_pension_salary_basis,
                            exchange,
                        )
                        .maximum_sacrifice;
                    entry.salary_exchange = Some(exchange);
                } else {
                    entry.salary_exchange = None;
                }
            }
            let Some(exchange) = &mut entry.salary_exchange else {
                ui.label(
                    egui::RichText::new("The complete one-time payment remains taxable salary.")
                        .small()
                        .color(secondary_text()),
                );
                return;
            };

            ui.add_space(6.0);
            let mut use_previous_year_basis =
                exchange.previous_year_pension_salary_basis.is_some();
            if ui
                .checkbox(
                    &mut use_previous_year_basis,
                    "Use previous year's pensionable salary for the 35% ceiling",
                )
                .on_hover_text(
                    "Uses a fixed preceding-year salary basis. The basis and 35% ceiling then do not decrease with this salary exchange.",
                )
                .changed()
            {
                let mut current_year_exchange = *exchange;
                current_year_exchange.previous_year_pension_salary_basis = None;
                let suggested_basis = context
                    .allowance_for(
                        entry.amount,
                        entry.included_in_pension_salary_basis,
                        current_year_exchange,
                    )
                    .pension_salary_basis_before;
                exchange.previous_year_pension_salary_basis =
                    use_previous_year_basis.then_some(suggested_basis);
            }
            if let Some(previous_year_basis) = &mut exchange.previous_year_pension_salary_basis {
                ui.label(secondary_label("Previous year's pensionable salary"));
                ui.add_sized(
                    [ui.available_width().min(180.0), 28.0],
                    egui::DragValue::new(previous_year_basis)
                        .range(0..=MAX_INCOME)
                        .suffix(" SEK")
                        .speed(1_000.0),
                );
            }

            ui.add_space(6.0);
            let mut use_actual_prior_costs = exchange
                .pension_and_insurance_costs_before_exchange
                .is_some();
            if ui
                .checkbox(
                    &mut use_actual_prior_costs,
                    "Use actual pension and insurance costs before this exchange",
                )
                .on_hover_text(
                    "Replaces the app's estimated regular, vacation, and other pension contributions with the employer's confirmed total.",
                )
                .changed()
            {
                let mut calculated_exchange = *exchange;
                calculated_exchange.pension_and_insurance_costs_before_exchange = None;
                let suggested_costs = context
                    .allowance_for(
                        entry.amount,
                        entry.included_in_pension_salary_basis,
                        calculated_exchange,
                    )
                    .pension_contributions_before;
                exchange.pension_and_insurance_costs_before_exchange =
                    use_actual_prior_costs.then_some(suggested_costs);
            }
            if let Some(actual_costs) =
                &mut exchange.pension_and_insurance_costs_before_exchange
            {
                ui.label(secondary_label(
                    "Pension and insurance costs before exchange",
                ));
                ui.add_sized(
                    [ui.available_width().min(180.0), 28.0],
                    egui::DragValue::new(actual_costs)
                        .range(0..=MAX_INCOME)
                        .suffix(" SEK")
                        .speed(1_000.0),
                );
            }

            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(secondary_label("Employer uplift"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut exchange.employer_adds_uplift, "Added");
                    if exchange.employer_adds_uplift {
                        basis_points_percentage_editor(
                            ui,
                            "salary-exchange-uplift",
                            &mut exchange.uplift_basis_points,
                        );
                    }
                });
            });

            let allowance = context.allowance_for(
                entry.amount,
                entry.included_in_pension_salary_basis,
                *exchange,
            );
            let maximum_sacrifice = allowance.maximum_sacrifice;
            exchange.sacrificed_salary = exchange.sacrificed_salary.min(maximum_sacrifice);
            let allowance = context.allowance_for(
                entry.amount,
                entry.included_in_pension_salary_basis,
                *exchange,
            );

            ui.add_space(6.0);
            egui::Grid::new(("salary-exchange-allowance", entry.id))
                .num_columns(2)
                .striped(true)
                .min_col_width(220.0)
                .show(ui, |ui| {
                    if allowance.previous_year_pension_salary_basis.is_some() {
                        value_row(
                            ui,
                            "Previous-year pensionable salary (fixed)",
                            format_sek(allowance.pension_salary_basis_after),
                        );
                    } else {
                        value_row(
                            ui,
                            "Current-year pensionable salary before exchange",
                            format_sek(allowance.pension_salary_basis_before),
                        );
                        value_row(
                            ui,
                            "Current-year pensionable salary after exchange",
                            format_sek(allowance.pension_salary_basis_after),
                        );
                    }
                    value_row(
                        ui,
                        "35% contribution ceiling",
                        format_sek(allowance.ceiling),
                    );
                    if allowance
                        .pension_and_insurance_costs_before_exchange
                        .is_some()
                    {
                        value_row(
                            ui,
                            "Actual pension and insurance costs before exchange",
                            format_sek(allowance.pension_contributions_before),
                        );
                    } else {
                        value_row(
                            ui,
                            "Regular pension contributions",
                            format_sek(allowance.regular_pension_premiums),
                        );
                        if allowance.vacation_pension_premiums > 0 {
                            value_row(
                                ui,
                                "Vacation-payout pension contribution",
                                format_sek(allowance.vacation_pension_premiums),
                            );
                        }
                        if allowance.other_exchange_contributions > 0 {
                            value_row(
                                ui,
                                "Other salary-exchange contributions",
                                format_sek(allowance.other_exchange_contributions),
                            );
                        }
                    }
                    value_row(
                        ui,
                        "Contribution room",
                        format_sek(allowance.available_contribution),
                    );
                    value_row(
                        ui,
                        "Pension and insurance costs for allowance",
                        format_sek(allowance.total_employer_pension_contributions),
                    );
                    value_row(
                        ui,
                        if allowance.previous_year_pension_salary_basis.is_some() {
                            "Share of previous-year pensionable salary"
                        } else {
                            "Share of current-year pensionable salary after exchange"
                        },
                        format!("{:.2}%", allowance.contribution_share_of_basis()),
                    );
                });
            let allowance_explanation = if allowance
                .previous_year_pension_salary_basis
                .is_some()
            {
                "Previous-year main-rule estimate: pension and insurance costs may be 35% of the fixed preceding-year pensionable salary, rounded down to whole SEK and capped at 592 000 SEK for 2026."
            } else {
                "Current-year main-rule estimate: pension and insurance costs may be 35% of pensionable salary after exchange, rounded down to whole SEK and capped at 592 000 SEK for 2026."
            };
            ui.label(
                egui::RichText::new(allowance_explanation)
                    .small()
                    .color(secondary_text()),
            );

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(secondary_label("Salary to exchange"));
                    ui.add_sized(
                        [170.0, 28.0],
                        egui::DragValue::new(&mut exchange.sacrificed_salary)
                            .range(0..=maximum_sacrifice)
                            .suffix(" SEK")
                            .speed(1_000.0),
                    );
                });
                if ui.button("Use maximum").clicked() {
                    exchange.sacrificed_salary = maximum_sacrifice;
                }
                ui.label(
                    egui::RichText::new(format!("Maximum: {}", format_sek(maximum_sacrifice)))
                        .small()
                        .color(secondary_text()),
                );
            });

            let pension_contribution = exchange.pension_contribution();
            let taxable_payment = entry.total_annual_amount();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Resulting pension contribution: {}",
                        format_sek(pension_contribution)
                    ))
                    .strong()
                    .color(green_color()),
                );
                ui.label("·");
                ui.label(
                    egui::RichText::new(format!(
                        "Taxable cash payment: {}",
                        format_sek(taxable_payment)
                    ))
                    .strong()
                    .color(primary_text()),
                );
            });
        });
}

pub(super) fn vacation_compensation_editor(
    ui: &mut egui::Ui,
    entry: &mut IncomeEntry,
    period_changed: bool,
) {
    if period_changed && let Some(vacation) = &mut entry.vacation_compensation {
        vacation.payout_days = VacationCompensation::suggested_days(
            vacation.annual_entitlement_days,
            entry.start,
            entry.end,
        );
    }

    egui::Frame::new()
        .fill(egui::Color32::from_rgb(246, 249, 248))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            let mut annual_days = entry
                .vacation_compensation
                .map(|vacation| vacation.annual_entitlement_days)
                .unwrap_or(0);
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.label(secondary_label(
                        "Vacation days per full year (0 means none)",
                    ));
                    let response = ui.add_sized(
                        [95.0, 28.0],
                        egui::DragValue::new(&mut annual_days)
                            .range(0..=100)
                            .suffix(" days"),
                    );
                    if response.changed() {
                        entry.vacation_compensation = (annual_days > 0).then(|| {
                            VacationCompensation::suggested(
                                annual_days,
                                entry.start,
                                entry.end,
                            )
                        });
                    }
                });

                let Some(vacation) = &mut entry.vacation_compensation else {
                    return;
                };
                columns[1].vertical(|ui| {
                    ui.label(secondary_label("Days paid out"));
                    ui.add_sized(
                        [95.0, 28.0],
                        egui::DragValue::new(&mut vacation.payout_days)
                            .range(0..=365)
                            .suffix(" days"),
                    );
                });
            });

            let Some(mut vacation) = entry.vacation_compensation else {
                ui.label(
                    egui::RichText::new(
                        "Enter the annual entitlement to estimate accrued vacation compensation.",
                    )
                    .small()
                    .color(secondary_text()),
                );
                return;
            };
            if ui.small_button("Use suggested days").clicked() {
                vacation.payout_days = VacationCompensation::suggested_days(
                    vacation.annual_entitlement_days,
                    entry.start,
                    entry.end,
                );
            }
            ui.add_space(6.0);
            ui.label(secondary_label("Compensation rate per paid day"));
            ui.horizontal(|ui| {
                basis_points_percentage_editor(
                    ui,
                    "vacation-compensation-rate",
                    &mut vacation.rate_basis_points,
                );
            });
            let per_day = vacation.amount_per_day(entry.amount);
            ui.label(
                egui::RichText::new(format!(
                    "Monthly salary × {}% = {per_day:.2} SEK/day",
                    format_basis_points_percentage(vacation.rate_basis_points),
                ))
                .small()
                .color(secondary_text()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Vacation compensation: {}",
                    format_sek(vacation.amount(entry.amount)),
                ))
                .strong()
                .color(primary_text()),
            );
            ui.add_space(5.0);
            ui.checkbox(
                &mut vacation.included_in_pension_salary_basis,
                "Treat this vacation payout as pensionable",
            )
            .on_hover_text(
                "Enabled by default: the ITP1-style estimate treats paid vacation compensation as pensionable when it is paid.",
            );
            if vacation.included_in_pension_salary_basis {
                let benchmark = vacation.additional_benchmark_pension_premium(entry.amount);
                ui.label(format!(
                    "Estimated additional employer pension contribution: {}",
                    format_sek(benchmark)
                ));
                let mut use_override = vacation.pension_premium_override.is_some();
                if ui
                    .checkbox(
                        &mut use_override,
                        "Use actual pension contribution for this vacation payout",
                    )
                    .changed()
                {
                    vacation.pension_premium_override = use_override.then_some(benchmark);
                }
                if let Some(actual) = &mut vacation.pension_premium_override {
                    ui.label(secondary_label(
                        "Actual pension contribution for vacation payout",
                    ));
                    ui.add_sized(
                        [ui.available_width().min(180.0), 28.0],
                        egui::DragValue::new(actual)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(100.0),
                    );
                }
            } else {
                vacation.pension_premium_override = None;
            }
            entry.vacation_compensation = Some(vacation);
        });
}

pub(super) fn date_editor(ui: &mut egui::Ui, label: &str, id: u64, date: &mut Date2026) -> bool {
    let previous = *date;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(249, 251, 250))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(5.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(secondary_label(label));
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("date-month", id))
                        .selected_text(month_name(date.month))
                        .width(78.0)
                        .show_ui(ui, |ui| {
                            for month in 1..=12 {
                                ui.selectable_value(&mut date.month, month, month_name(month));
                            }
                        });
                    let maximum = Date2026::days_in_month(date.month);
                    date.day = date.day.clamp(1, maximum);
                    egui::ComboBox::from_id_salt(("date-day", id))
                        .selected_text(format!("Day {}", date.day))
                        .width(64.0)
                        .show_ui(ui, |ui| {
                            for day in 1..=maximum {
                                ui.selectable_value(&mut date.day, day, day.to_string());
                            }
                        });
                    ui.label(egui::RichText::new("2026").strong().color(secondary_text()));
                });
            });
        });
    *date != previous
}

pub(super) fn month_name(month: u8) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS[usize::from(month.clamp(1, 12) - 1)]
}

pub(super) fn percentage_editor(ui: &mut egui::Ui, id: &'static str, percent: &mut u32) {
    ui.push_id(id, |ui| {
        egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, border_color()))
            .corner_radius(5.0)
            .inner_margin(3.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .add_sized([28.0, 26.0], egui::Button::new("−"))
                        .on_hover_text("Decrease by 1%")
                        .clicked()
                    {
                        *percent = percent.saturating_sub(1);
                    }
                    ui.add_sized(
                        [72.0, 26.0],
                        egui::DragValue::new(percent)
                            .range(0..=100)
                            .suffix(" %")
                            .speed(1.0),
                    );
                    if ui
                        .add_sized([28.0, 26.0], egui::Button::new("+"))
                        .on_hover_text("Increase by 1%")
                        .clicked()
                    {
                        *percent = percent.saturating_add(1).min(100);
                    }
                });
            });
    });
}

pub(super) fn basis_points_percentage_editor(
    ui: &mut egui::Ui,
    id: &'static str,
    basis_points: &mut u32,
) {
    exact_basis_points_percentage_editor(ui, id, basis_points, 10_000);
}

pub(super) fn exact_basis_points_percentage_editor(
    ui: &mut egui::Ui,
    id: &'static str,
    basis_points: &mut u32,
    maximum_basis_points: u32,
) {
    ui.push_id(id, |ui| {
        let editor_id = ui.make_persistent_id("percentage");
        let text_id = editor_id.with("text");
        let mut text = ui
            .ctx()
            .data(|data| data.get_temp::<String>(text_id))
            .unwrap_or_else(|| format_basis_points_percentage(*basis_points));

        let response = ui.add_sized(
            [74.0, 26.0],
            egui::TextEdit::singleline(&mut text)
                .id(editor_id)
                .horizontal_align(egui::Align::RIGHT),
        );
        if response.changed() {
            text = sanitize_percentage_text(&text);
            if let Some(parsed) = parse_basis_points_percentage(&text) {
                *basis_points = parsed.min(maximum_basis_points);
                if parsed > maximum_basis_points {
                    text = format_basis_points_percentage(*basis_points);
                }
            }
        }
        if !response.has_focus() {
            text = format_basis_points_percentage(*basis_points);
        }
        ui.label("%");

        ui.ctx().data_mut(|data| data.insert_temp(text_id, text));
    });
}

pub(super) fn income_eligibility(kind: IncomeKind) -> &'static str {
    match (kind.is_pgi_eligible(), kind.is_sgi_eligible()) {
        (true, true) => "PGI eligible · SGI estimate eligible",
        (true, false) => "PGI eligible · does not establish ongoing SGI",
        _ => "PGI — · SGI —",
    }
}

pub(super) fn eligibility_badge(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .small()
        .strong()
        .color(green_color())
}

pub(super) fn withholding_rule_text(rule: AppliedWithholding) -> String {
    match rule {
        AppliedWithholding::ActualAmount => "entered actual amount".to_owned(),
        AppliedWithholding::Table(column) => format!("table column {}", column as u8),
        AppliedWithholding::TableAndOneTime(column, percent) => format!(
            "table column {} plus one-time-payment table at {percent}%",
            column as u8,
        ),
        AppliedWithholding::OneTimeTable(percent) => {
            format!("one-time-payment table at {percent}%")
        }
        AppliedWithholding::Secondary30 => {
            format!("secondary payer at {SECONDARY_WITHHOLDING_PERCENT}%")
        }
        AppliedWithholding::AdjustmentPercent(percent) => {
            format!("percentage jämkning at {percent}%")
        }
        AppliedWithholding::None => "no preliminary withholding".to_owned(),
    }
}
