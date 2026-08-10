use super::*;

pub(super) fn dividend_allowance_editor(ui: &mut egui::Ui, plan: &mut IncomePlan) {
    let current_year_own_company_salary = plan.own_company_sourced_work_income();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Preliminary 2027 dividend allowance")
                        .strong()
                        .size(16.0)
                        .color(primary_text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Dividend paid in 2027 using your company’s short 2026 first year",
                    )
                    .small()
                    .color(secondary_text()),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.label(
                    egui::RichText::new("Income year 2027 · K10 in 2028")
                        .small()
                        .strong()
                        .color(yellow_text())
                        .background_color(egui::Color32::from_rgb(252, 246, 225)),
                );
            });
        });
        ui.add_space(10.0);

        if ui.available_width() >= 760.0 {
            ui.columns(2, |columns| {
                dividend_ownership_inputs(&mut columns[0], &mut plan.dividend_allowance);
                dividend_payroll_inputs(&mut columns[1], &mut plan.dividend_allowance);
            });
            ui.add_space(8.0);
            dividend_capital_inputs(ui, &mut plan.dividend_allowance);
        } else {
            dividend_ownership_inputs(ui, &mut plan.dividend_allowance);
            ui.add_space(8.0);
            dividend_payroll_inputs(ui, &mut plan.dividend_allowance);
            ui.add_space(8.0);
            dividend_capital_inputs(ui, &mut plan.dividend_allowance);
        }

        ui.add_space(10.0);
        match plan.dividend_allowance_2027() {
            Ok(allowance) => {
                ui.columns(3, |columns| {
                    compact_fact(
                        &mut columns[0],
                        "Max dividend at 20%",
                        format_sek(allowance.total),
                    );
                    compact_fact(
                        &mut columns[1],
                        "Personal tax if fully used",
                        format_sek(allowance.tax_at_twenty_percent()),
                    );
                    compact_fact(
                        &mut columns[2],
                        "Net if fully used",
                        format_sek(allowance.net_after_twenty_percent_tax()),
                    );
                });

                ui.add_space(8.0);
                egui::Grid::new("dividend-allowance-breakdown")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(240.0)
                    .show(ui, |ui| {
                        value_row(
                            ui,
                            "Ownership-adjusted basic amount",
                            format_sek(allowance.basic_amount),
                        );
                        value_row(
                            ui,
                            "Your marked 2026 company salary",
                            format_sek(allowance.owner_cash_salary),
                        );
                        value_row(
                            ui,
                            "Company/group payroll used",
                            format_sek(allowance.company_cash_payroll),
                        );
                        value_row(
                            ui,
                            &format!(
                                "Wage basis after {} deduction",
                                format_sek(DIVIDEND_WAGE_DEDUCTION_2027)
                            ),
                            format_sek(allowance.joint_wage_basis_after_deduction),
                        );
                        value_row(
                            ui,
                            "Your wage-based allowance before cap",
                            format_sek(allowance.wage_allowance_before_cap),
                        );
                        value_row(
                            ui,
                            "50× owner/related salary cap",
                            format_sek(allowance.wage_cap),
                        );
                        value_row(
                            ui,
                            "Wage-based allowance used",
                            format_sek(allowance.wage_allowance),
                        );
                        value_row(
                            ui,
                            "Acquisition-cost interest",
                            format_sek(allowance.acquisition_cost_interest),
                        );
                        value_row(
                            ui,
                            "Saved dividend allowance",
                            format_sek(allowance.saved_allowance),
                        );
                        value_row(ui, "2027 tax gränsbelopp", format_sek(allowance.total));
                    });

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Maximum 2027 dividend eligible for 20% tax: {}.",
                        format_sek(allowance.total),
                    ))
                    .strong()
                    .color(green_color()),
                );
                ui.label(
                    egui::RichText::new(
                        "For a normal privately held Swedish AB, no preliminary personal tax is normally withheld from the dividend. The company pays the full dividend, reports it, and you declare it on K10 in 2028; any remaining personal tax is settled through your final-tax account.",
                    )
                    .small()
                    .color(secondary_text()),
                );
            }
            Err(issue) => {
                ui.colored_label(
                    egui::Color32::DARK_RED,
                    dividend_allowance_issue_text(issue),
                );
            }
        }

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "2026 cash salary marked as own-company sourced: {}. This feeds the 2027 owner-salary and one-person payroll calculation directly.",
                format_sek(current_year_own_company_salary),
            ))
            .small()
            .color(secondary_text()),
        );
        ui.label(
            egui::RichText::new(
                "The salary total includes cash salary, one-time salary, and vacation compensation after salary exchange. Pension income, benefits, and pension contributions are excluded.",
            )
            .small()
            .color(secondary_text()),
        );
        ui.label(
            egui::RichText::new(
                "This is the tax allowance only. The actual dividend also requires sufficient free equity in the adopted 2026 balance sheet, a prudence assessment, and a shareholder-meeting decision; bank balance alone is not enough.",
            )
            .small()
            .color(secondary_text()),
        );
    });
}

pub(super) fn dividend_ownership_inputs(
    ui: &mut egui::Ui,
    inputs: &mut DividendAllowanceInputs2027,
) {
    input_group(ui, "Ownership at 1 January 2027", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(secondary_label("Your share in this company"));
            basis_points_percentage_editor(
                ui,
                "dividend-own-share",
                &mut inputs.ownership_basis_points,
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(secondary_label("Spouse share in this company"));
            basis_points_percentage_editor(
                ui,
                "dividend-spouse-share",
                &mut inputs.spouse_ownership_basis_points,
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(secondary_label(
                "Your summed shares in other qualified companies",
            ));
            multi_company_ownership_editor(ui, &mut inputs.other_qualified_ownership_basis_points);
        });
    });
}

pub(super) fn input_group(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(249, 251, 250))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(title).strong().color(primary_text()));
            ui.add_space(4.0);
            add_contents(ui);
        });
}

pub(super) fn dividend_payroll_inputs(ui: &mut egui::Ui, inputs: &mut DividendAllowanceInputs2027) {
    input_group(ui, "Cash compensation paid in 2026", |ui| {
        ui.checkbox(
            &mut inputs.one_person_company,
            "One-person company — marked salary is the total payroll",
        );
        if inputs.one_person_company {
            ui.label(
                egui::RichText::new(
                    "Your owner salary and total payroll are derived from marked 2026 salary rows.",
                )
                .small()
                .strong()
                .color(blue_color()),
            );
        } else {
            dividend_sek_input(
                ui,
                "Company/group payroll — all employees",
                &mut inputs.company_cash_payroll_2026,
            );
            dividend_sek_input(
                ui,
                "Highest related person's salary",
                &mut inputs.highest_related_cash_salary_2026,
            );
        }
        ui.label(
            egui::RichText::new(
                "Total payroll includes qualifying cash compensation to all employees in the company and qualifying subsidiaries. Benefits and pension contributions are excluded.",
            )
            .small()
            .color(secondary_text()),
        );
        ui.label(
            egui::RichText::new(format!(
                "At 100% ownership, payroll must exceed {} before the wage-based allowance becomes positive.",
                format_sek(DIVIDEND_WAGE_DEDUCTION_2027),
            ))
            .small()
            .strong()
            .color(blue_color()),
        );
    });
}

pub(super) fn dividend_capital_inputs(ui: &mut egui::Ui, inputs: &mut DividendAllowanceInputs2027) {
    input_group(ui, "Capital and carried allowance", |ui| {
        ui.horizontal_wrapped(|ui| {
            dividend_sek_input(ui, "Acquisition cost", &mut inputs.acquisition_cost);
            dividend_sek_input(ui, "Saved allowance", &mut inputs.saved_allowance);
        });
        ui.label(
            egui::RichText::new(format!(
                "Only acquisition cost above {} earns interest. The exact 2027 rate uses the government borrowing rate on 30 November 2026 plus 9%, so it is not known yet.",
                format_sek(DIVIDEND_ACQUISITION_COST_THRESHOLD),
            ))
            .small()
            .color(secondary_text()),
        );
    });
}

pub(super) fn dividend_sek_input(ui: &mut egui::Ui, label: &str, value: &mut u32) {
    ui.vertical(|ui| {
        ui.label(secondary_label(label));
        ui.add(
            egui::DragValue::new(value)
                .range(0..=MAX_INCOME)
                .suffix(" SEK")
                .speed(1_000.0),
        );
    });
}

pub(super) fn multi_company_ownership_editor(ui: &mut egui::Ui, basis_points: &mut u32) {
    exact_basis_points_percentage_editor(ui, "other-qualified-ownership", basis_points, 1_000_000);
}

pub(super) fn dividend_allowance_issue_text(issue: DividendAllowanceIssue) -> &'static str {
    match issue {
        DividendAllowanceIssue::OwnershipExceedsOneHundredPercent => {
            "Your ownership in one company cannot exceed 100%."
        }
        DividendAllowanceIssue::SpouseOwnershipExceedsCompany => {
            "Your and your spouse's combined ownership cannot exceed 100%."
        }
        DividendAllowanceIssue::PersonalSalaryExceedsCompanyPayroll => {
            "Owner or related-person salary cannot exceed the total company/group payroll."
        }
        DividendAllowanceIssue::MissingAcquisitionCostInterestRate => {
            "The exact 2027 acquisition-cost interest rate is not known until the government borrowing rate for 30 November 2026 has been established."
        }
    }
}
