use eframe::egui;
use swedish_tax::{
    AnnualTax, AppliedWithholding, Date2026, EntryWithholding, IncomeBasisEstimate, IncomeEntry,
    IncomeKind, IncomePlan, MAX_TAX_TABLE, MIN_TAX_TABLE, PayerRole, RegularPensionPremium,
    SalaryExchange, TaxAgeGroup, TaxColumn, TaxDeduction, VacationCompensation,
    annual_tax_for_income_profile, estimated_sgi_progress_for_income, monthly_deduction,
    public_pension_progress_for_income,
};

const MAX_INCOME: u32 = 100_000_000;
const DEFAULT_MONTHLY_INCOME: u32 = 660_400 / 12;
type HoverHelp = fn(&mut egui::Ui);
type Summary<'a> = (
    &'a str,
    String,
    Option<String>,
    egui::Color32,
    Option<HoverHelp>,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AdjustmentCalibration {
    basis_income: u32,
    percent: u32,
    formula_tax_at_basis: u32,
    assumed_tax_at_basis: u32,
    implied_tax_adjustment: i64,
    projected_ordinary_tax: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Calculation {
    monthly_income: u32,
    annual_income: u32,
    ordinary_income: u32,
    dividend_income: u32,
    table_deduction: TaxDeduction,
    annual_tax: AnnualTax,
    adjustment_calibration: Option<AdjustmentCalibration>,
    ordinary_final_tax: u32,
    dividend_tax: u32,
    total_tax: u32,
    withheld_tax: u32,
    regular_pension_premiums: u32,
    vacation_pension_premiums: u32,
    salary_exchange_sacrifice: u32,
    salary_exchange_pension_contributions: u32,
    employer_pension_contributions: u32,
    marginal_rate: f64,
    pension_progress: IncomeBasisEstimate,
    sgi_progress: IncomeBasisEstimate,
}

impl Calculation {
    fn new(table: u8, age_group: TaxAgeGroup, plan: &IncomePlan) -> Option<Self> {
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
                    let basis_profile = swedish_tax::AnnualIncomeProfile {
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
        let dividend_tax = percentage(totals.dividend_income, 20);
        let total_tax = ordinary_final_tax.saturating_add(dividend_tax);
        let withheld_tax = plan.estimated_withholding(table, age_group).total;
        let upper_profile = swedish_tax::AnnualIncomeProfile {
            work_income: totals.work_income.saturating_add(12_000),
            pension_income: totals.pension_income,
        };
        let upper_tax = annual_tax_for_income_profile(table, age_group, upper_profile)?.total;
        let marginal_rate = (f64::from(upper_tax) - f64::from(annual_tax.total)) * 100.0 / 12_000.0;
        let pension_progress = public_pension_progress_for_income(totals.work_income);
        let sgi_progress = estimated_sgi_progress_for_income(totals.sgi_annual_rate);

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
            pension_progress,
            sgi_progress,
        })
    }

    fn table_reference_tax(self) -> u32 {
        match self.table_deduction {
            TaxDeduction::Amount(amount) => amount,
            TaxDeduction::Percent(percent) => percentage(self.monthly_income, percent),
        }
    }

    fn table_reference_net(self) -> u32 {
        self.monthly_income
            .saturating_sub(self.table_reference_tax())
    }

    fn effective_rate(self) -> f64 {
        if self.ordinary_income == 0 {
            0.0
        } else {
            f64::from(self.ordinary_final_tax) * 100.0 / f64::from(self.ordinary_income)
        }
    }

    const fn annual_net(self) -> u32 {
        self.annual_income.saturating_sub(self.total_tax)
    }

    const fn cash_after_withholding(self) -> u32 {
        self.annual_income.saturating_sub(self.withheld_tax)
    }

    fn tax_balance(self) -> i64 {
        i64::from(self.total_tax) - i64::from(self.withheld_tax)
    }
}

struct TaxApp {
    table: u8,
    age_group: TaxAgeGroup,
    income_plan: IncomePlan,
    income_editor_open: bool,
    income_entry_to_reveal: Option<u64>,
}

#[derive(Clone, Copy)]
struct PensionEditorContext {
    regular_pension_premiums: u32,
    vacation_pension_premiums: u32,
    total_exchange_contributions: u32,
    pension_salary_basis: u32,
}

impl Default for TaxApp {
    fn default() -> Self {
        Self {
            table: 32,
            age_group: TaxAgeGroup::Under66AtYearStart,
            income_plan: IncomePlan::with_monthly_salary(DEFAULT_MONTHLY_INCOME),
            income_editor_open: false,
            income_entry_to_reveal: None,
        }
    }
}

impl TaxApp {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&context.egui_ctx);
        Self::default()
    }

    fn add_income_for_editing(&mut self) -> u64 {
        let id = self.income_plan.add_entry(IncomeKind::AnnualSalary);
        self.income_entry_to_reveal = Some(id);
        id
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(surface_color())
            .stroke(egui::Stroke::new(1.0, border_color()))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Inputs")
                        .strong()
                        .size(15.0)
                        .color(primary_text()),
                );
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.label(secondary_label("Tax table"));
                        let response = egui::ComboBox::from_id_salt("tax-table")
                            .selected_text(self.table.to_string())
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
                                    ui.selectable_value(&mut self.table, table, table.to_string());
                                }
                            })
                            .response;
                        response.on_hover_ui(table_selector_help);
                    });

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(secondary_label("Age at start of 2026"));
                        egui::ComboBox::from_id_salt("tax-age-group")
                            .selected_text(match self.age_group {
                                TaxAgeGroup::Under66AtYearStart => "Under 66",
                                TaxAgeGroup::AtLeast66AtYearStart => "66 or older",
                            })
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.age_group,
                                    TaxAgeGroup::Under66AtYearStart,
                                    "Under 66",
                                );
                                ui.selectable_value(
                                    &mut self.age_group,
                                    TaxAgeGroup::AtLeast66AtYearStart,
                                    "66 or older",
                                );
                            });
                    });

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        let simple_monthly = has_uniform_monthly_table_reference(&self.income_plan);
                        let totals = self.income_plan.totals();
                        ui.label(secondary_label(if simple_monthly {
                            "Monthly income"
                        } else {
                            "Annual income plan"
                        }));
                        if ui
                            .add_sized(
                                [210.0, 28.0],
                                egui::Button::new(format!(
                                    "{} {}  ·  Edit…",
                                    format_sek(if simple_monthly {
                                        totals.monthly_taxable_income()
                                    } else {
                                        totals.gross_income()
                                    }),
                                    if simple_monthly { "/ month" } else { "/ year" }
                                )),
                            )
                            .clicked()
                        {
                            self.income_editor_open = true;
                        }
                    });
                });
            });
    }

    fn results(&self, ui: &mut egui::Ui, calculation: Calculation) {
        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Tax result")
                    .strong()
                    .size(19.0)
                    .color(primary_text()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Table {} / salary column {} / pension column {}",
                        self.table,
                        self.age_group.salary_column() as u8,
                        self.age_group.pension_column() as u8,
                    ))
                    .color(blue_color()),
                );
            });
        });
        ui.add_space(10.0);

        let summaries = [
            (
                if calculation.adjustment_calibration.is_some() {
                    "Jämkning-calibrated tax projection"
                } else {
                    "Final tax estimate"
                },
                format_sek(calculation.total_tax),
                Some(format!("Marginal tax: {:.1}%", calculation.marginal_rate)),
                green_color(),
                Some(marginal_rate_help as HoverHelp),
            ),
            (
                "Calculated withholding",
                format_sek(calculation.withheld_tax),
                Some(format!(
                    "Cash after withholding: {}",
                    format_sek(calculation.cash_after_withholding())
                )),
                blue_color(),
                None,
            ),
            (
                "Annual net after final tax",
                format_sek(calculation.annual_net()),
                Some(tax_balance_summary(calculation.tax_balance())),
                primary_text(),
                None,
            ),
        ];
        summary_tiles(ui, &summaries);

        ui.add_space(24.0);
        annual_reconciliation(ui, calculation);

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(18.0);
        monthly_table_reference(ui, calculation, self.table, self.age_group.salary_column());

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(18.0);
        income_basis_ceiling_progress(ui, calculation);

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(18.0);
        ui.label(
            egui::RichText::new(if calculation.adjustment_calibration.is_some() {
                "Annual tax projection breakdown"
            } else {
                "Annual formula breakdown"
            })
            .strong()
            .size(17.0)
            .color(primary_text()),
        );
        ui.add_space(8.0);
        annual_breakdown(ui, calculation.annual_tax);
        if let Some(calibration) = calculation.adjustment_calibration {
            ui.add_space(8.0);
            adjustment_calibration_breakdown(ui, calibration);
        }
        if calculation.dividend_income > 0 {
            ui.add_space(8.0);
            egui::Grid::new("dividend-tax-breakdown-grid")
                .num_columns(2)
                .striped(true)
                .min_col_width(260.0)
                .show(ui, |ui| {
                    value_row(
                        ui,
                        "Own-AB dividend",
                        format_sek(calculation.dividend_income),
                    );
                    value_row(
                        ui,
                        "Dividend tax at 20%",
                        format_sek(calculation.dividend_tax),
                    );
                    value_row(ui, "Total final tax", format_sek(calculation.total_tax));
                });
        }
    }

    fn income_editor(&mut self, context: &egui::Context) {
        if !self.income_editor_open {
            return;
        }

        let mut open = self.income_editor_open;
        let mut close_requested = false;
        egui::Window::new("Income calculator")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(920.0)
            .min_width(760.0)
            .show(context, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Edit an existing row directly. Add another income only for a separate source or payment.",
                    )
                    .color(secondary_text()),
                );
                ui.add_space(10.0);
                adjustment_editor(ui, &mut self.income_plan);
                ui.add_space(12.0);

                let mut remove_id = None;
                let adjustment_percent = self.income_plan.adjustment_percent;
                let pension_totals = self.income_plan.totals();
                let pension_context = PensionEditorContext {
                    regular_pension_premiums: pension_totals.regular_pension_premiums,
                    vacation_pension_premiums: pension_totals.vacation_pension_premiums,
                    total_exchange_contributions: pension_totals
                        .salary_exchange_pension_contributions,
                    pension_salary_basis: pension_totals.pension_salary_basis,
                };
                let entry_withholding = self
                    .income_plan
                    .estimated_withholding(self.table, self.age_group)
                    .entries;
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        for entry in &mut self.income_plan.entries {
                            let reveal = self.income_entry_to_reveal == Some(entry.id);
                            let withholding = entry_withholding
                                .iter()
                                .find(|estimate| estimate.entry_id == entry.id)
                                .copied();
                            let entry_response = ui.push_id(("income-entry", entry.id), |ui| {
                                income_entry_editor(
                                    ui,
                                    entry,
                                    adjustment_percent,
                                    withholding,
                                    pension_context,
                                    reveal,
                                );
                                if ui
                                    .small_button("Remove this income")
                                    .on_hover_text("Remove this row")
                                    .clicked()
                                {
                                    remove_id = Some(entry.id);
                                }
                            });
                            if reveal {
                                entry_response
                                    .response
                                    .scroll_to_me(Some(egui::Align::TOP));
                                self.income_entry_to_reveal = None;
                            }
                            ui.add_space(8.0);
                        }
                    });
                if let Some(id) = remove_id {
                    self.income_plan.remove_entry(id);
                }

                ui.horizontal(|ui| {
                    if ui.button("+ Add another income").clicked() {
                        self.add_income_for_editing();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Done").clicked() {
                            close_requested = true;
                        }
                    });
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                let withholding = self
                    .income_plan
                    .estimated_withholding(self.table, self.age_group);
                income_calculation_breakdown(ui, &self.income_plan, withholding.total);
            });

        self.income_editor_open = open && !close_requested;
    }
}

fn table_selector_help(ui: &mut egui::Ui) {
    ui.set_max_width(360.0);
    ui.label(egui::RichText::new("Find your tax table").strong());
    ui.add_space(4.0);
    ui.label(
        "On skatteverket.se, open Mina sidor and select A-skattsedel, skattetabell och \
         jämkningsbeslut. Open your A-tax certificate as a PDF; it states which table your \
         payer should use.",
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "The table is based on where you were registered on 1 November of the preceding year.",
        )
        .small()
        .color(secondary_text()),
    );
}

fn marginal_rate_help(ui: &mut egui::Ui) {
    ui.set_max_width(380.0);
    ui.label(egui::RichText::new("How marginal tax is calculated").strong());
    ui.add_space(4.0);
    ui.label("Marginal tax estimates the tax on a potential next 1,000 SEK of monthly income.");
    ui.add_space(4.0);
    ui.label(
        "This app annualizes your current monthly income and an income 1,000 SEK higher, then \
         calculates the annual tax for both using the formula.",
    );
    ui.add_space(4.0);
    ui.label("The additional annual tax is divided by 12,000 and shown as a percentage.");
}

fn adjustment_editor(ui: &mut egui::Ui, plan: &mut IncomePlan) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(246, 249, 248))
        .stroke(egui::Stroke::new(1.0, border_color()))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                let mut enabled = plan.adjustment_percent.is_some();
                if ui
                    .checkbox(&mut enabled, "Existing percentage jämkning")
                    .changed()
                {
                    plan.adjustment_percent = enabled.then_some(30);
                }
                if let Some(percent) = &mut plan.adjustment_percent {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(secondary_label("Decision withholding"));
                        percentage_editor(ui, "adjustment-percentage", percent);
                    });
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(
                            "Choose ‘shown to this payer’ on every affected income row.",
                        )
                        .small()
                        .color(secondary_text()),
                    );
                }
            });
        });
}

fn income_calculation_breakdown(ui: &mut egui::Ui, plan: &IncomePlan, withheld_tax: u32) {
    let totals = plan.totals();
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Annual overview")
                    .strong()
                    .size(16.0)
                    .color(primary_text()),
            );
            ui.label(
                egui::RichText::new(
                    "Each editable income appears once. Employer pension contributions are separate from cash income.",
                )
                .small()
                .color(secondary_text()),
            );
            ui.add_space(6.0);

            egui::Grid::new("income-entry-summary")
                .num_columns(5)
                .striped(true)
                .min_col_width(112.0)
                .show(ui, |ui| {
                    ui.label(secondary_label("Income entry"));
                    ui.label(secondary_label("Gross cash"));
                    ui.label(secondary_label("Salary exchange"));
                    ui.label(secondary_label("Cash income"));
                    ui.label(secondary_label("Employer pension"));
                    ui.end_row();

                    for (index, entry) in plan.entries.iter().enumerate() {
                        let name = if entry.description.trim().is_empty() {
                            format!("Entry {}", index + 1)
                        } else {
                            entry.description.trim().to_owned()
                        };
                        let vacation = entry.vacation_compensation_amount();
                        let gross_cash = entry.annual_amount().saturating_add(vacation);
                        let exchange = entry.salary_exchange_sacrifice();
                        let cash_income = entry.total_annual_amount();
                        let employer_pension = entry
                            .regular_pension_premium_amount()
                            .saturating_add(entry.vacation_pension_premium_amount())
                            .saturating_add(entry.salary_exchange_pension_contribution());

                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(name).strong());
                            ui.label(
                                egui::RichText::new(entry.kind.label())
                                    .small()
                                    .color(secondary_text()),
                            );
                            if entry.kind.is_monthly() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {} to {} {}",
                                        month_name(entry.start.month),
                                        entry.start.day,
                                        month_name(entry.end.month),
                                        entry.end.day,
                                    ))
                                    .small()
                                    .color(secondary_text()),
                                );
                            }
                            if vacation > 0 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Includes {} vacation compensation",
                                        format_sek(vacation)
                                    ))
                                    .small()
                                    .color(secondary_text()),
                                );
                            }
                        });
                        ui.label(format_sek(gross_cash));
                        ui.label(if exchange > 0 {
                            format!("−{}", format_sek(exchange))
                        } else {
                            "—".to_owned()
                        });
                        ui.label(format_sek(cash_income));
                        ui.label(if employer_pension > 0 {
                            format_sek(employer_pension)
                        } else {
                            "—".to_owned()
                        });
                        ui.end_row();
                    }

                    let total_gross_cash = totals
                        .gross_income()
                        .saturating_add(totals.salary_exchange_sacrifice);
                    ui.label(egui::RichText::new("Total").strong());
                    ui.label(egui::RichText::new(format_sek(total_gross_cash)).strong());
                    ui.label(
                        egui::RichText::new(if totals.salary_exchange_sacrifice > 0 {
                            format!(
                                "−{}",
                                format_sek(totals.salary_exchange_sacrifice)
                            )
                        } else {
                            "—".to_owned()
                        })
                        .strong(),
                    );
                    ui.label(egui::RichText::new(format_sek(totals.gross_income())).strong());
                    ui.label(
                        egui::RichText::new(format_sek(
                            totals.total_employer_pension_contributions(),
                        ))
                        .strong(),
                    );
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Cash-income totals used by the tax calculation")
                    .strong()
                    .color(primary_text()),
            );
            egui::Grid::new("income-tax-category-totals")
                .num_columns(2)
                .striped(true)
                .min_col_width(280.0)
                .show(ui, |ui| {
                    value_row(
                        ui,
                        "Salary and compensation",
                        format_sek(totals.work_income),
                    );
                    if totals.pension_income > 0 {
                        value_row(ui, "Tjänstepension received", format_sek(totals.pension_income));
                    }
                    if totals.dividend_income > 0 {
                        value_row(ui, "Own-AB dividend", format_sek(totals.dividend_income));
                    }
                    value_row(ui, "Total cash income", format_sek(totals.gross_income()));
                    value_row(ui, "Calculated tax withheld", format_sek(withheld_tax));
                });
        });
}

fn income_entry_editor(
    ui: &mut egui::Ui,
    entry: &mut IncomeEntry,
    adjustment: Option<u32>,
    withholding: Option<EntryWithholding>,
    pension_context: PensionEditorContext,
    focus_description: bool,
) {
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let previous_kind = entry.kind;
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(secondary_label("Description"));
                    let description_response = ui.add_sized(
                        [190.0, 26.0],
                        egui::TextEdit::singleline(&mut entry.description)
                            .hint_text("Employer or payment"),
                    );
                    if focus_description {
                        description_response.request_focus();
                    }
                });
                ui.vertical(|ui| {
                    ui.label(secondary_label("Income type"));
                    egui::ComboBox::from_id_salt(("income-kind", entry.id))
                        .selected_text(entry.kind.label())
                        .width(270.0)
                        .show_ui(ui, |ui| {
                            for kind in IncomeKind::ALL {
                                ui.selectable_value(&mut entry.kind, kind, kind.label());
                            }
                        });
                });
                ui.vertical(|ui| {
                    ui.label(secondary_label(if entry.kind.is_monthly() {
                        "Amount per month"
                    } else {
                        "Annual / one-time amount"
                    }));
                    ui.add(
                        egui::DragValue::new(&mut entry.amount)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(1_000.0),
                    );
                });
            });

            if entry.kind != previous_kind {
                entry.included_in_pension_salary_basis = matches!(
                    entry.kind,
                    IncomeKind::AnnualSalary | IncomeKind::MonthlySalary
                );
                if matches!(
                    entry.kind,
                    IncomeKind::AnnualSalary | IncomeKind::MonthlySalary
                ) && entry.regular_pension_premium.is_none()
                {
                    entry.regular_pension_premium = Some(RegularPensionPremium::default());
                }
                if entry.kind != IncomeKind::OneTimeSalary {
                    entry.salary_exchange = None;
                }
                if entry.kind != IncomeKind::MonthlySalary {
                    entry.vacation_compensation = None;
                }
                if !matches!(
                    entry.kind,
                    IncomeKind::AnnualSalary | IncomeKind::MonthlySalary
                ) {
                    entry.use_full_year_projection_as_adjustment_basis = false;
                }
            }

            if entry.kind.is_monthly() {
                ui.add_space(8.0);
                ui.label(secondary_label("Payment period"));
                let mut period_changed = false;
                ui.horizontal_wrapped(|ui| {
                    period_changed |= date_editor(ui, "From", entry.id, &mut entry.start);
                    ui.label(egui::RichText::new("to").color(secondary_text()));
                    period_changed |=
                        date_editor(ui, "Through", entry.id + 1_000_000, &mut entry.end);
                    if ui.small_button("Full year").clicked() {
                        entry.start = Date2026::new(1, 1);
                        entry.end = Date2026::new(12, 31);
                        period_changed = true;
                    }
                    if !entry.is_valid() {
                        ui.colored_label(
                            egui::Color32::DARK_RED,
                            "End date must follow start date",
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
            ui.horizontal_wrapped(|ui| {
                if entry.kind.is_dividend() {
                    ui.label(eligibility_badge(
                        "20% final tax within gränsbelopp · 0% withheld · no PGI · no SGI",
                    ));
                } else {
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(secondary_label("Payer"));
                            egui::ComboBox::from_id_salt(("payer-role", entry.id))
                                .selected_text(match entry.payer_role {
                                    PayerRole::Main => "Main payer",
                                    PayerRole::Secondary => "Secondary payer",
                                })
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut entry.payer_role,
                                        PayerRole::Main,
                                        "Main payer — table",
                                    );
                                    ui.selectable_value(
                                        &mut entry.payer_role,
                                        PayerRole::Secondary,
                                        "Secondary payer — 30%",
                                    );
                                });
                            if adjustment.is_some() {
                                ui.checkbox(
                                    &mut entry.adjustment_applies,
                                    "Jämkning shown to this payer",
                                );
                            } else {
                                entry.adjustment_applies = false;
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            let mut custom = entry.custom_withholding_percent.is_some();
                            if ui.checkbox(&mut custom, "Custom withholding").changed() {
                                entry.custom_withholding_percent = custom.then_some(30);
                            }
                            if let Some(percent) = &mut entry.custom_withholding_percent {
                                percentage_editor(ui, "custom-withholding", percent);
                            }
                            ui.label(eligibility_badge(income_eligibility(entry.kind)));
                        });
                    });
                }
            });
            if let Some(withholding) = withholding {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Calculated withholding: {} · {}",
                        format_sek(withholding.withheld),
                        withholding_rule_text(withholding.rule),
                    ))
                    .small()
                    .color(secondary_text()),
                );
            }
        });
}

fn adjustment_basis_editor(ui: &mut egui::Ui, entry: &mut IncomeEntry, percent: u32) {
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

fn regular_pension_premium_editor(ui: &mut egui::Ui, entry: &mut IncomeEntry) {
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
                "Include salary in current-year pension salary basis",
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

            let monthly_salary = if entry.kind == IncomeKind::AnnualSalary {
                entry.amount / 12
            } else {
                entry.amount
            };
            let benchmark = RegularPensionPremium::benchmark_monthly(monthly_salary);
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new(
                    "2026 individual-pension benchmark: 4.5% through 52 125 SEK/month, then 30%.",
                )
                .small()
                .color(secondary_text()),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Benchmark: {}", format_sek(benchmark)));
                let mut use_override = premium.monthly_override.is_some();
                if ui
                    .checkbox(&mut use_override, "Use actual monthly contribution")
                    .changed()
                {
                    premium.monthly_override = use_override.then_some(benchmark);
                }
                if let Some(actual) = &mut premium.monthly_override {
                    ui.add_sized(
                        [145.0, 28.0],
                        egui::DragValue::new(actual)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(100.0),
                    );
                }
            });
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

fn salary_exchange_editor(
    ui: &mut egui::Ui,
    entry: &mut IncomeEntry,
    context: PensionEditorContext,
) {
    let contribution_before_edit = entry.salary_exchange_pension_contribution();
    let other_exchange_contributions = context
        .total_exchange_contributions
        .saturating_sub(contribution_before_edit);
    let used_before_this_payment = context
        .regular_pension_premiums
        .saturating_add(context.vacation_pension_premiums)
        .saturating_add(other_exchange_contributions);
    let current_basis_after_exchange = entry.pension_salary_basis_amount();
    let other_pension_salary_basis = context
        .pension_salary_basis
        .saturating_sub(current_basis_after_exchange);

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
                "Include this payment in current-year pension salary basis",
            )
            .on_hover_text(
                "Off by default for a termination payment. Turn it on if the employment agreement treats this cash payment as pensionable salary.",
            );
            let payment_in_basis = if entry.included_in_pension_salary_basis {
                entry.amount
            } else {
                0
            };
            let pension_salary_basis_before =
                other_pension_salary_basis.saturating_add(payment_in_basis);
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
                    exchange.sacrificed_salary = exchange.maximum_sacrifice(
                        entry.amount,
                        pension_salary_basis_before,
                        used_before_this_payment,
                        entry.included_in_pension_salary_basis,
                    );
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

            let maximum_sacrifice = exchange.maximum_sacrifice(
                entry.amount,
                pension_salary_basis_before,
                used_before_this_payment,
                entry.included_in_pension_salary_basis,
            );
            exchange.sacrificed_salary = exchange.sacrificed_salary.min(maximum_sacrifice);
            let sacrifice_in_basis = if entry.included_in_pension_salary_basis {
                exchange.sacrificed_salary
            } else {
                0
            };
            let pension_salary_basis_after =
                pension_salary_basis_before.saturating_sub(sacrifice_in_basis);
            let ceiling = SalaryExchange::allowance_ceiling(pension_salary_basis_after);
            let available_contribution = ceiling.saturating_sub(used_before_this_payment);

            ui.add_space(6.0);
            egui::Grid::new(("salary-exchange-allowance", entry.id))
                .num_columns(2)
                .striped(true)
                .min_col_width(220.0)
                .show(ui, |ui| {
                    value_row(
                        ui,
                        "Current-year pension salary before exchange",
                        format_sek(pension_salary_basis_before),
                    );
                    value_row(
                        ui,
                        "Current-year pension salary after exchange",
                        format_sek(pension_salary_basis_after),
                    );
                    value_row(
                        ui,
                        "Indicative employer deduction ceiling after exchange",
                        format_sek(ceiling),
                    );
                    value_row(
                        ui,
                        "Regular pension contributions",
                        format_sek(context.regular_pension_premiums),
                    );
                    if context.vacation_pension_premiums > 0 {
                        value_row(
                            ui,
                            "Vacation-payout pension contribution",
                            format_sek(context.vacation_pension_premiums),
                        );
                    }
                    if other_exchange_contributions > 0 {
                        value_row(
                            ui,
                            "Other salary-exchange contributions",
                            format_sek(other_exchange_contributions),
                        );
                    }
                    value_row(
                        ui,
                        "Available pension contribution",
                        format_sek(available_contribution),
                    );
                });
            ui.label(
                egui::RichText::new(
                    "Current-year main-rule estimate: total employer pension contributions may be 35% of pension salary after exchange, capped at 592 000 SEK for 2026.",
                )
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
            let taxable_payment = entry.amount.saturating_sub(exchange.sacrificed_salary);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Employer pension contribution: {}",
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

fn vacation_compensation_editor(ui: &mut egui::Ui, entry: &mut IncomeEntry, period_changed: bool) {
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
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
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
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(secondary_label("Days paid out"));
                    ui.add_sized(
                        [95.0, 28.0],
                        egui::DragValue::new(&mut vacation.payout_days)
                            .range(0..=365)
                            .suffix(" days"),
                    );
                });
                if ui.small_button("Use suggested days").clicked() {
                    vacation.payout_days = VacationCompensation::suggested_days(
                        vacation.annual_entitlement_days,
                        entry.start,
                        entry.end,
                    );
                }
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
            ui.add_space(6.0);
            let per_day = f64::from(entry.amount) / 21.0 + f64::from(entry.amount) * 0.0043;
            ui.label(
                egui::RichText::new(format!(
                    "Same-year statutory estimate: monthly salary / 21 + 0.43% of monthly salary = {per_day:.2} SEK/day"
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
                "Include vacation payout in pension salary basis",
            )
            .on_hover_text(
                "Enabled by default: the ITP1-style estimate treats paid vacation compensation as pensionable when it is paid.",
            );
            if vacation.included_in_pension_salary_basis {
                let vacation_amount = vacation.amount(entry.amount);
                let benchmark = RegularPensionPremium::benchmark_monthly(
                    entry.amount.saturating_add(vacation_amount),
                )
                .saturating_sub(RegularPensionPremium::benchmark_monthly(entry.amount));
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "Estimated additional employer pension contribution: {}",
                        format_sek(benchmark)
                    ));
                    let mut use_override = vacation.pension_premium_override.is_some();
                    if ui
                        .checkbox(&mut use_override, "Use actual contribution")
                        .changed()
                    {
                        vacation.pension_premium_override = use_override.then_some(benchmark);
                    }
                    if let Some(actual) = &mut vacation.pension_premium_override {
                        ui.add_sized(
                            [145.0, 28.0],
                            egui::DragValue::new(actual)
                                .range(0..=MAX_INCOME)
                                .suffix(" SEK")
                                .speed(100.0),
                        );
                    }
                });
            } else {
                vacation.pension_premium_override = None;
            }
            entry.vacation_compensation = Some(vacation);
        });
}

fn date_editor(ui: &mut egui::Ui, label: &str, id: u64, date: &mut Date2026) -> bool {
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
                    ui.add_sized(
                        [54.0, 28.0],
                        egui::DragValue::new(&mut date.day)
                            .range(1..=maximum)
                            .prefix("Day ")
                            .speed(1.0),
                    );
                    ui.label(egui::RichText::new("2026").strong().color(secondary_text()));
                });
            });
        });
    *date != previous
}

fn month_name(month: u8) -> &'static str {
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

fn percentage_editor(ui: &mut egui::Ui, id: &'static str, percent: &mut u32) {
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

fn basis_points_percentage_editor(ui: &mut egui::Ui, id: &'static str, basis_points: &mut u32) {
    ui.push_id(id, |ui| {
        let mut percent = f64::from(*basis_points) / 100.0;
        let response = ui.add_sized(
            [90.0, 26.0],
            egui::DragValue::new(&mut percent)
                .range(0.0..=100.0)
                .suffix(" %")
                .speed(0.05)
                .max_decimals(2),
        );
        if response.changed() {
            *basis_points = (percent * 100.0).round() as u32;
        }
    });
}

fn income_eligibility(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::AnnualSalary | IncomeKind::MonthlySalary => {
            "PGI eligible · SGI estimate eligible"
        }
        IncomeKind::OneTimeSalary => "PGI eligible · does not establish ongoing SGI",
        IncomeKind::MonthlyOccupationalPension | IncomeKind::AnnualOccupationalPension => {
            "PGI — · SGI —"
        }
        IncomeKind::OwnCompanyDividend => "PGI — · SGI —",
    }
}

fn has_uniform_monthly_table_reference(plan: &IncomePlan) -> bool {
    let [entry] = plan.entries.as_slice() else {
        return false;
    };
    if entry.payer_role != PayerRole::Main
        || entry.adjustment_applies
        || entry.custom_withholding_percent.is_some()
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

fn eligibility_badge(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .small()
        .strong()
        .color(green_color())
}

fn withholding_rule_text(rule: AppliedWithholding) -> String {
    match rule {
        AppliedWithholding::Table(column) => format!("table column {}", column as u8),
        AppliedWithholding::TableAndOneTime(column, percent) => format!(
            "table column {} plus one-time-payment table at {percent}%",
            column as u8,
        ),
        AppliedWithholding::OneTimeTable(percent) => {
            format!("one-time-payment table at {percent}%")
        }
        AppliedWithholding::Secondary30 => "secondary payer at 30%".to_owned(),
        AppliedWithholding::AdjustmentPercent(percent) => {
            format!("percentage jämkning at {percent}%")
        }
        AppliedWithholding::CustomPercent(percent) => format!("custom {percent}%"),
        AppliedWithholding::None => "no preliminary withholding".to_owned(),
    }
}

impl eframe::App for TaxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, background_color());

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(24.0).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Swedish Tax 2026")
                                    .strong()
                                    .size(26.0)
                                    .color(primary_text()),
                            );
                            ui.label(
                                egui::RichText::new("Preliminary income tax")
                                    .size(14.0)
                                    .color(secondary_text()),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(
                                egui::RichText::new("Income year 2026")
                                    .strong()
                                    .color(yellow_text()),
                            );
                        });
                    });
                    ui.add_space(18.0);

                    self.controls(ui);
                    if let Some(calculation) =
                        Calculation::new(self.table, self.age_group, &self.income_plan)
                    {
                        self.results(ui, calculation);
                    } else {
                        ui.add_space(20.0);
                        ui.colored_label(
                            egui::Color32::DARK_RED,
                            "Complete or correct the dated income rows to show results.",
                        );
                    }

                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(
                            "Preliminary tax based on Skatteverket tables and SKV 433, edition 36.",
                        )
                        .small()
                        .color(secondary_text()),
                    );
                });
            });

        self.income_editor(ui.ctx());
    }
}

fn configure_style(context: &egui::Context) {
    context.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = background_color();
    visuals.window_fill = surface_color();
    visuals.extreme_bg_color = egui::Color32::WHITE;
    visuals.faint_bg_color = egui::Color32::from_rgb(238, 242, 240);
    visuals.selection.bg_fill = blue_color();
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.hyperlink_color = blue_color();
    context.set_visuals_of(egui::Theme::Light, visuals);
    context.style_mut_of(egui::Theme::Light, |style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
    });
}

fn summary_tiles(ui: &mut egui::Ui, summaries: &[Summary<'_>; 3]) {
    if ui.available_width() >= 720.0 {
        ui.columns(3, |columns| {
            for (column, summary) in columns.iter_mut().zip(summaries) {
                summary_tile(
                    column,
                    summary.0,
                    &summary.1,
                    summary.2.as_deref(),
                    summary.3,
                    summary.4,
                );
            }
        });
    } else {
        for summary in summaries {
            summary_tile(
                ui,
                summary.0,
                &summary.1,
                summary.2.as_deref(),
                summary.3,
                summary.4,
            );
            ui.add_space(6.0);
        }
    }
}

fn summary_tile(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    detail: Option<&str>,
    color: egui::Color32,
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
            ui.label(egui::RichText::new(value).strong().size(20.0).color(color));
            if let Some(detail) = detail {
                ui.add_space(4.0);
                let response = ui.label(
                    egui::RichText::new(detail)
                        .strong()
                        .size(13.0)
                        .color(primary_text()),
                );
                if let Some(help) = detail_help {
                    response.on_hover_ui(help);
                }
            }
        });
}

fn annual_reconciliation(ui: &mut egui::Ui, calculation: Calculation) {
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
                    if pension_component_count > 1 {
                        "Total employer pension contributions"
                    } else {
                        "Employer pension contributions"
                    },
                    format_sek(calculation.employer_pension_contributions),
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
            let balance = calculation.tax_balance();
            if balance > 0 {
                value_row(ui, "Expected kvarskatt", format_sek(balance as u32));
            } else if balance < 0 {
                value_row(
                    ui,
                    "Expected refund",
                    format_sek(balance.unsigned_abs() as u32),
                );
            } else {
                value_row(ui, "Expected balance", format_sek(0));
            }
        });
}

fn monthly_table_reference(
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
                "Cash after table deduction",
                format_sek(calculation.table_reference_net()),
            );
        });
}

fn income_basis_ceiling_progress(ui: &mut egui::Ui, calculation: Calculation) {
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

fn income_basis_row(
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

fn income_basis_help(ui: &mut egui::Ui) {
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

fn annual_breakdown(ui: &mut egui::Ui, tax: AnnualTax) {
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

fn adjustment_calibration_breakdown(ui: &mut egui::Ui, calibration: AdjustmentCalibration) {
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

fn value_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(egui::RichText::new(label).color(secondary_text()));
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new(value).strong().color(primary_text()));
    });
    ui.end_row();
}

fn table_deduction_text(deduction: TaxDeduction) -> String {
    match deduction {
        TaxDeduction::Amount(amount) => format!("{} / month", format_sek(amount)),
        TaxDeduction::Percent(percent) => format!("{percent}% of payment"),
    }
}

fn percentage(amount: u32, percent: u32) -> u32 {
    (u64::from(amount) * u64::from(percent) / 100).min(u64::from(u32::MAX)) as u32
}

fn tax_balance_summary(balance: i64) -> String {
    match balance.cmp(&0) {
        std::cmp::Ordering::Greater => {
            format!("Expected kvarskatt: {}", format_sek(balance as u32))
        }
        std::cmp::Ordering::Less => format!(
            "Expected refund: {}",
            format_sek(balance.unsigned_abs() as u32)
        ),
        std::cmp::Ordering::Equal => "Withholding matches final tax".to_owned(),
    }
}

fn format_credit(value: u32) -> String {
    if value == 0 {
        format_sek(0)
    } else {
        format!("-{}", format_sek(value))
    }
}

fn format_sek(value: u32) -> String {
    format!("{} SEK", grouped_digits(value))
}

fn grouped_digits(value: u32) -> String {
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

fn secondary_label(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .small()
        .strong()
        .color(secondary_text())
}

fn background_color() -> egui::Color32 {
    egui::Color32::from_rgb(244, 247, 246)
}

fn surface_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 255, 255)
}

fn border_color() -> egui::Color32 {
    egui::Color32::from_rgb(210, 218, 215)
}

fn primary_text() -> egui::Color32 {
    egui::Color32::from_rgb(30, 44, 41)
}

fn secondary_text() -> egui::Color32 {
    egui::Color32::from_rgb(91, 105, 101)
}

fn blue_color() -> egui::Color32 {
    egui::Color32::from_rgb(0, 82, 147)
}

fn green_color() -> egui::Color32 {
    egui::Color32::from_rgb(24, 121, 78)
}

fn yellow_text() -> egui::Color32 {
    egui::Color32::from_rgb(128, 91, 0)
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1_040.0, 780.0])
            .with_min_inner_size([620.0, 560.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Swedish Tax 2026",
        native_options,
        Box::new(|context| Ok(Box::new(TaxApp::new(context)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_32_is_selected_by_default() {
        assert_eq!(TaxApp::default().table, 32);
    }

    #[test]
    fn newly_added_income_is_marked_for_reveal_and_editing() {
        let mut app = TaxApp::default();
        let id = app.add_income_for_editing();

        assert_eq!(app.income_entry_to_reveal, Some(id));
        assert_eq!(app.income_plan.entries.last().unwrap().id, id);
    }

    #[test]
    fn monthly_table_reference_requires_one_uniform_full_year_salary() {
        let mut plan = IncomePlan::with_monthly_salary(93_000);
        assert!(has_uniform_monthly_table_reference(&plan));

        plan.entries[0].end = Date2026::new(10, 18);
        assert!(!has_uniform_monthly_table_reference(&plan));

        plan = IncomePlan::with_annual_salary(1_116_000);
        assert!(has_uniform_monthly_table_reference(&plan));

        plan.add_entry(IncomeKind::AnnualOccupationalPension);
        assert!(!has_uniform_monthly_table_reference(&plan));
    }

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
    fn default_income_is_the_highest_whole_monthly_income_below_the_state_tax_breakpoint() {
        let app = TaxApp::default();
        let calculation = Calculation::new(app.table, app.age_group, &app.income_plan).unwrap();

        assert_eq!(calculation.monthly_income, 55_033);
        assert_eq!(calculation.annual_income, 660_396);
        assert_eq!(calculation.annual_tax.state_income_tax, 0);
        assert!(calculation.monthly_income * 12 <= 660_400);
        assert!((calculation.monthly_income + 1) * 12 > 660_400);
    }

    #[test]
    fn annual_salary_uses_one_twelfth_for_the_monthly_table_reference() {
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

    #[test]
    fn detailed_income_scenario_drives_tax_pgi_sgi_and_withholding() {
        let mut plan = IncomePlan::with_annual_salary(0);
        let salary = &mut plan.entries[0];
        salary.kind = IncomeKind::MonthlySalary;
        salary.amount = 93_000;
        salary.start = Date2026::new(1, 1);
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
    fn dividend_adds_fixed_final_tax_without_default_withholding() {
        let mut plan = IncomePlan::with_annual_salary(420_000);
        let dividend_id = plan.add_entry(IncomeKind::OwnCompanyDividend);
        plan.entries
            .iter_mut()
            .find(|entry| entry.id == dividend_id)
            .unwrap()
            .amount = 200_000;
        let calculation = Calculation::new(32, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.annual_income, 620_000);
        assert_eq!(calculation.dividend_tax, 40_000);
        assert_eq!(calculation.total_tax, calculation.annual_tax.total + 40_000);
        assert_eq!(
            calculation.withheld_tax,
            IncomePlan::with_annual_salary(420_000)
                .estimated_withholding(32, TaxAgeGroup::Under66AtYearStart)
                .total
        );
    }

    #[test]
    fn marginal_rate_uses_annual_formula() {
        let plan = IncomePlan::with_annual_salary(216_000);
        let calculation = Calculation::new(34, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        let expected = f64::from(38_894 - 35_889) * 100.0 / 12_000.0;
        assert_eq!(calculation.marginal_rate, expected);
    }

    #[test]
    fn zero_income_has_zero_tax_and_a_stable_rate() {
        let plan = IncomePlan::with_annual_salary(0);
        let calculation = Calculation::new(33, TaxAgeGroup::Under66AtYearStart, &plan).unwrap();

        assert_eq!(calculation.table_deduction, TaxDeduction::Amount(0));
        assert_eq!(calculation.annual_tax.total, 0);
        assert_eq!(calculation.withheld_tax, 0);
        assert_eq!(calculation.table_reference_net(), 0);
        assert_eq!(calculation.effective_rate(), 0.0);
    }

    #[test]
    fn every_published_table_is_available() {
        let plan = IncomePlan::with_annual_salary(420_000);
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            assert!(
                Calculation::new(table, TaxAgeGroup::Under66AtYearStart, &plan).is_some(),
                "table {table}"
            );
        }
    }

    #[test]
    fn formatting_groups_sek_without_locale_dependencies() {
        assert_eq!(format_sek(0), "0 SEK");
        assert_eq!(format_sek(1_234_567), "1 234 567 SEK");
        assert_eq!(tax_balance_summary(2_400), "Expected kvarskatt: 2 400 SEK");
        assert_eq!(tax_balance_summary(-350), "Expected refund: 350 SEK");
    }
}
