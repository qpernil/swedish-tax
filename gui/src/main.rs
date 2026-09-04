mod dividend_ui;
mod income_ui;
mod results_ui;

use dividend_ui::*;
use eframe::egui;
use income_ui::*;
use results_ui::*;
use swedish_tax::{
    AdjustmentCalibration, AnnualTax, AppliedWithholding, Calculation,
    DIVIDEND_ACQUISITION_COST_THRESHOLD, DIVIDEND_BASIC_AMOUNT_2027, DIVIDEND_TAX_PERCENT,
    DIVIDEND_WAGE_DEDUCTION_2027, Date2026, DividendAllowanceInputs2027, DividendAllowanceIssue,
    EntryWithholding, IncomeBasisEstimate, IncomeEntry, IncomeKind, IncomePlan,
    IncomePlanValidationIssue, IncomeTaxCategory, MAX_TAX_TABLE, MIN_TAX_TABLE, PayerRole,
    PersistedAppState, RegularPensionPremium, SECONDARY_WITHHOLDING_PERCENT, SalaryExchange,
    SalaryExchangeContext, TaxAgeGroup, TaxBalance, TaxColumn, TaxDeduction, VacationCompensation,
    WithholdingSummary,
};

const MAX_INCOME: u32 = 100_000_000;
const APP_STATE_STORAGE_KEY: &str = "swedish-tax-app-state";
type HoverHelp = fn(&mut egui::Ui);

struct Summary<'a> {
    label: &'a str,
    value: String,
    detail: Option<String>,
    value_color: egui::Color32,
    detail_color: egui::Color32,
    detail_help: Option<HoverHelp>,
}

struct TaxApp {
    table: u8,
    age_group: TaxAgeGroup,
    income_plan: IncomePlan,
    income_editor_open: bool,
    calculation_trace_open: bool,
    selected_income_entry: Option<u64>,
}

impl Default for TaxApp {
    fn default() -> Self {
        Self::from_persisted_state(PersistedAppState::default())
    }
}

impl TaxApp {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&context.egui_ctx);
        context
            .storage
            .and_then(|storage| {
                eframe::get_value::<PersistedAppState>(storage, APP_STATE_STORAGE_KEY)
            })
            .filter(PersistedAppState::is_supported)
            .map(Self::from_persisted_state)
            .unwrap_or_default()
    }

    fn from_persisted_state(state: PersistedAppState) -> Self {
        Self {
            table: state.table,
            age_group: state.age_group,
            selected_income_entry: state.income_plan.entries.first().map(|entry| entry.id),
            income_plan: state.income_plan,
            income_editor_open: false,
            calculation_trace_open: false,
        }
    }

    fn persisted_state(&self) -> PersistedAppState {
        PersistedAppState::new(self.table, self.age_group, self.income_plan.clone())
    }

    fn add_income_for_editing(&mut self) -> u64 {
        self.add_income_kind_for_editing(IncomeKind::AnnualSalary)
    }

    fn add_income_kind_for_editing(&mut self, kind: IncomeKind) -> u64 {
        let id = self.income_plan.add_entry(kind);
        self.selected_income_entry = Some(id);
        self.income_editor_open = true;
        id
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() >= 820.0 {
            ui.columns(2, |columns| {
                self.tax_settings_card(&mut columns[0]);
                self.income_plan_card(&mut columns[1]);
            });
        } else {
            self.tax_settings_card(ui);
            ui.add_space(12.0);
            self.income_plan_card(ui);
        }
    }

    fn tax_settings_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.set_min_height(170.0);
            card_heading(
                ui,
                "Tax settings",
                "Tax table, age-dependent columns, and jämkning",
            );
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(secondary_label("Tax table"));
                    let response = egui::ComboBox::from_id_salt("tax-table")
                        .selected_text(format!("Table {}", self.table))
                        .width(105.0)
                        .show_ui(ui, |ui| {
                            for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
                                ui.selectable_value(
                                    &mut self.table,
                                    table,
                                    format!("Table {table}"),
                                );
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
                        .width(125.0)
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
                adjustment_editor(ui, &mut self.income_plan);
            });
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!(
                    "Salary column {} · pension column {}",
                    self.age_group.salary_column() as u8,
                    self.age_group.pension_column() as u8,
                ))
                .small()
                .strong()
                .color(blue_color()),
            );
        });
    }

    fn income_plan_card(&mut self, ui: &mut egui::Ui) {
        let withholding = self
            .income_plan
            .estimated_withholding(self.table, self.age_group);
        let totals = self.income_plan.totals();
        card(ui, |ui| {
            ui.set_min_height(170.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Income plan")
                            .strong()
                            .size(16.0)
                            .color(primary_text()),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} income row{}",
                            self.income_plan.entries.len(),
                            if self.income_plan.entries.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ))
                        .small()
                        .color(secondary_text()),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    let mut kind_to_add = None;
                    ui.menu_button("+ Add", |ui| {
                        for kind in IncomeKind::ALL {
                            if ui.button(income_kind_short(kind)).clicked() {
                                kind_to_add = Some(kind);
                                ui.close();
                            }
                        }
                    });
                    if let Some(kind) = kind_to_add {
                        self.add_income_kind_for_editing(kind);
                    }
                });
            });
            ui.add_space(8.0);

            egui::Grid::new("home-income-plan")
                .num_columns(2)
                .striped(true)
                .min_col_width(130.0)
                .show(ui, |ui| {
                    for (index, entry) in self.income_plan.entries.iter().enumerate() {
                        let response = ui.vertical(|ui| {
                            let response = ui.selectable_label(
                                false,
                                egui::RichText::new(income_entry_name(entry, index)).strong(),
                            );
                            ui.label(
                                egui::RichText::new(income_kind_short(entry.kind))
                                    .small()
                                    .color(secondary_text()),
                            );
                            response
                        });
                        if response.inner.clicked() {
                            self.selected_income_entry = Some(entry.id);
                            self.income_editor_open = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(format_sek(entry.total_annual_amount()))
                                        .strong(),
                                );
                                let withheld = withholding
                                    .entries
                                    .iter()
                                    .find(|row| row.entry_id == entry.id)
                                    .map(|row| row.withheld)
                                    .unwrap_or(0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Withheld {}",
                                        format_sek(withheld)
                                    ))
                                    .small()
                                    .color(blue_color()),
                                );
                            });
                        });
                        ui.end_row();
                    }
                });

            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Total cash income").strong());
                ui.label(
                    egui::RichText::new(format_sek(totals.gross_income()))
                        .strong()
                        .color(primary_text()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Edit full plan…").clicked() {
                        self.income_editor_open = true;
                    }
                });
            });
            ui.label(
                egui::RichText::new(format!(
                    "Salary {} · pension {} · dividend {}",
                    format_sek(totals.work_income),
                    format_sek(totals.pension_income),
                    format_sek(totals.dividend_income),
                ))
                .small()
                .color(secondary_text()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Modeled tjänstepension contributions {} · {:.2}% of current-year pensionable salary after exchange",
                    format_sek(totals.total_employer_pension_contributions()),
                    totals.employer_pension_share_of_basis(),
                ))
                .small()
                .strong()
                .color(green_color()),
            );
        });
    }

    fn results(&mut self, ui: &mut egui::Ui, calculation: Calculation) {
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
            Summary {
                label: if calculation.adjustment_calibration.is_some() {
                    "Jämkning-calibrated tax projection"
                } else {
                    "Final tax estimate"
                },
                value: format_sek(calculation.total_tax),
                detail: Some(format!("Marginal tax: {:.1}%", calculation.marginal_rate)),
                value_color: green_color(),
                detail_color: primary_text(),
                detail_help: Some(marginal_rate_help as HoverHelp),
            },
            Summary {
                label: "Calculated withholding",
                value: format_sek(calculation.withheld_tax),
                detail: Some(format!(
                    "Cash after withholding: {}",
                    format_sek(calculation.cash_after_withholding())
                )),
                value_color: blue_color(),
                detail_color: primary_text(),
                detail_help: None,
            },
            Summary {
                label: "Annual net after final tax",
                value: format_sek(calculation.annual_net()),
                detail: Some(tax_balance_summary(calculation.tax_balance_outcome())),
                value_color: primary_text(),
                detail_color: tax_balance_color(calculation.tax_balance_outcome()),
                detail_help: None,
            },
        ];
        summary_tiles(ui, &summaries);

        ui.add_space(16.0);
        card(ui, |ui| annual_reconciliation(ui, calculation));
        ui.add_space(12.0);
        if ui
            .add_sized(
                [ui.available_width(), 34.0],
                egui::Button::new("Show calculation trace…"),
            )
            .clicked()
        {
            self.calculation_trace_open = true;
        }
        self.calculation_trace_window(ui.ctx(), calculation);

        ui.add_space(16.0);
        card(ui, |ui| {
            monthly_table_reference(ui, calculation, self.table, self.age_group.salary_column());
        });

        ui.add_space(16.0);
        card(ui, |ui| income_basis_ceiling_progress(ui, calculation));

        ui.add_space(16.0);
        card(ui, |ui| {
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
                            &format!("Dividend tax at {DIVIDEND_TAX_PERCENT}%"),
                            format_sek(calculation.dividend_tax),
                        );
                        value_row(ui, "Total final tax", format_sek(calculation.total_tax));
                    });
            }
        });
    }

    fn calculation_trace_window(&mut self, context: &egui::Context, calculation: Calculation) {
        if !self.calculation_trace_open {
            return;
        }

        let mut open = self.calculation_trace_open;
        egui::Window::new("Calculation trace")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(780.0)
            .default_height(680.0)
            .min_width(560.0)
            .min_height(420.0)
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("calculation-trace-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        calculation_trace(
                            ui,
                            &self.income_plan,
                            calculation,
                            self.table,
                            self.age_group,
                        );
                    });
            });
        self.calculation_trace_open = open;
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
            .default_width(1120.0)
            .default_height(780.0)
            .min_width(900.0)
            .min_height(620.0)
            .show(context, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Build the year from income rows. Select a row to edit it and see exactly how it is used.",
                    )
                    .color(secondary_text()),
                );
                ui.add_space(10.0);
                if !self.income_plan.entries.iter().any(|entry| {
                    self.selected_income_entry == Some(entry.id)
                }) {
                    self.selected_income_entry =
                        self.income_plan.entries.first().map(|entry| entry.id);
                }

                let withholding = self
                    .income_plan
                    .estimated_withholding(self.table, self.age_group);
                income_overview_table(
                    ui,
                    &self.income_plan,
                    &withholding,
                    &mut self.selected_income_entry,
                );
                ui.horizontal(|ui| {
                    if ui.button("+ Add income").clicked() {
                        self.add_income_for_editing();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Apply and close").clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.add_space(10.0);

                let adjustment_percent = self.income_plan.adjustment_percent;
                let selected_id = self.selected_income_entry;
                let pension_context = selected_id
                    .and_then(|id| self.income_plan.salary_exchange_context(id))
                    .unwrap_or_default();
                let mut remove_selected = false;
                let detail_height = (ui.available_height() - 150.0).max(130.0);
                egui::ScrollArea::vertical()
                    .id_salt("income-detail-scroll")
                    .max_height(detail_height)
                    .show(ui, |ui| {
                        ui.columns(2, |columns| {
                            if let Some(entry) = self
                                .income_plan
                                .entries
                                .iter_mut()
                                .find(|entry| Some(entry.id) == selected_id)
                            {
                                columns[0].label(
                                    egui::RichText::new("Selected income")
                                        .strong()
                                        .size(16.0)
                                        .color(primary_text()),
                                );
                                columns[0].add_space(6.0);
                                income_entry_editor(
                                    &mut columns[0],
                                    entry,
                                    adjustment_percent,
                                    withholding
                                        .entries
                                        .iter()
                                        .find(|estimate| estimate.entry_id == entry.id)
                                        .copied(),
                                    pension_context,
                                    false,
                                );
                                columns[0].add_space(6.0);
                                remove_selected = columns[0]
                                    .small_button("Remove selected income")
                                    .on_hover_text("Remove this income row")
                                    .clicked();
                            }

                            columns[1].label(
                                egui::RichText::new("Used in calculations")
                                    .strong()
                                    .size(16.0)
                                    .color(primary_text()),
                            );
                            columns[1].add_space(6.0);
                            selected_income_impact(
                                &mut columns[1],
                                &self.income_plan,
                                selected_id,
                                self.table,
                                self.age_group,
                            );
                        });
                    });

                if remove_selected && let Some(id) = selected_id {
                    self.income_plan.remove_entry(id);
                    self.selected_income_entry =
                        self.income_plan.entries.first().map(|entry| entry.id);
                }

                ui.add_space(10.0);
                income_totals_footer(
                    ui,
                    &self.income_plan,
                    self.table,
                    self.age_group,
                );
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
    let mut enabled = plan.adjustment_percent.is_some();
    let shown_count = plan
        .entries
        .iter()
        .filter(|entry| {
            !entry.kind.is_dividend()
                && entry.adjustment_applies
                && entry.actual_withholding.is_none()
        })
        .count();
    let overridden_count = plan
        .entries
        .iter()
        .filter(|entry| {
            !entry.kind.is_dividend()
                && entry.adjustment_applies
                && entry.actual_withholding.is_some()
        })
        .count();

    ui.vertical(|ui| {
        ui.label(secondary_label("Jämkning"));
        if ui
            .checkbox(&mut enabled, "Have decision")
            .on_hover_text(
                "Store the percentage from your Skatteverket decision. The main payer uses it by default; change that per payer below.",
            )
            .changed()
        {
            plan.set_adjustment_enabled(enabled);
        }
        if let Some(percent) = &mut plan.adjustment_percent {
            percentage_editor(ui, "adjustment-percentage", percent);
            let payer_status = if overridden_count > 0 {
                format!("{shown_count} applied · {overridden_count} overridden")
            } else {
                format!(
                    "{shown_count} payer{}",
                    if shown_count == 1 { "" } else { "s" }
                )
            };
            ui.label(
                egui::RichText::new(payer_status)
                    .small()
                    .color(secondary_text()),
            );
        }
    });
}

impl eframe::App for TaxApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let state_before_edit = self.persisted_state();
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
                                egui::RichText::new("Income plan and tax reconciliation")
                                    .size(14.0)
                                    .color(secondary_text()),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(
                                egui::RichText::new("2026")
                                    .strong()
                                    .color(yellow_text())
                                    .background_color(egui::Color32::from_rgb(252, 246, 225)),
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
                            validation_message(self.income_plan.validation_issue()),
                        );
                    }
                    ui.add_space(12.0);
                    dividend_allowance_editor(ui, &mut self.income_plan);

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

        let state_after_edit = self.persisted_state();
        if state_after_edit != state_before_edit
            && let Some(storage) = frame.storage_mut()
        {
            eframe::set_value(storage, APP_STATE_STORAGE_KEY, &state_after_edit);
            storage.flush();
        }
    }
}

fn validation_message(issue: Option<IncomePlanValidationIssue>) -> String {
    match issue {
        Some(IncomePlanValidationIssue::InvalidPaymentPeriod { .. }) => {
            "Check that each payment period's last day is on or after its first day.".to_owned()
        }
        Some(IncomePlanValidationIssue::SalaryExchangeExceedsAllowance { maximum, .. }) => format!(
            "Salary exchange exceeds the current maximum of {}. Open the income row and reduce it.",
            format_sek(maximum)
        ),
        None => "Check the income-plan values and try again.".to_owned(),
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
mod tests;
