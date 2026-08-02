use eframe::egui;
use swedish_tax::{
    AdjustmentCalibration, AnnualTax, AppliedWithholding, Calculation, DIVIDEND_TAX_PERCENT,
    Date2026, EntryWithholding, IncomeBasisEstimate, IncomeEntry, IncomeKind, IncomePlan,
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
                "Selected table and age-dependent columns",
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
                    "Total tjänstepension contribution {} · {:.2}% of pension salary after exchange",
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

fn adjustment_editor(ui: &mut egui::Ui, plan: &mut IncomePlan, table: u8, age_group: TaxAgeGroup) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(239, 247, 244))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(184, 211, 201),
        ))
        .corner_radius(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            let mut enabled = plan.adjustment_percent.is_some();
            let shown_count = plan
                .entries
                .iter()
                .filter(|entry| {
                    !entry.kind.is_dividend()
                        && entry.adjustment_applies
                        && entry.custom_withholding_percent.is_none()
                        && entry.actual_withholding.is_none()
                })
                .count();
            let overridden_count = plan
                .entries
                .iter()
                .filter(|entry| {
                    !entry.kind.is_dividend()
                        && entry.adjustment_applies
                        && (entry.custom_withholding_percent.is_some()
                            || entry.actual_withholding.is_some())
                })
                .count();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Jämkning")
                            .strong()
                            .size(16.0)
                            .color(primary_text()),
                    );
                    ui.label(
                        egui::RichText::new("Percentage decision and full-year calibration")
                            .small()
                            .color(secondary_text()),
                    );
                });
                if let Some(percent) = plan.adjustment_percent {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{percent}% · {}",
                                if shown_count == 0 {
                                    "no payer".to_owned()
                                } else {
                                    format!(
                                        "{shown_count} payer{}",
                                        if shown_count == 1 { "" } else { "s" }
                                    )
                                }
                            ))
                            .small()
                            .strong()
                            .color(yellow_text())
                            .background_color(egui::Color32::from_rgb(252, 246, 225)),
                        );
                    });
                }
            });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    if ui
                        .checkbox(&mut enabled, "Use a percentage jämkning decision")
                        .changed()
                    {
                        plan.set_adjustment_enabled(enabled);
                    }
                });
                if let Some(percent) = &mut plan.adjustment_percent {
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(secondary_label("Decision withholding"));
                        percentage_editor(ui, "adjustment-percentage", percent);
                    });
                }
            });

            let Some(percent) = plan.adjustment_percent else {
                ui.label(
                    egui::RichText::new(
                        "Enable this only when you have a percentage decision from Skatteverket.",
                    )
                    .small()
                    .color(secondary_text()),
                );
                return;
            };

            let totals = plan.totals();
            let projected_tax = Calculation::new(table, age_group, plan)
                .and_then(|calculation| calculation.adjustment_calibration)
                .map(|calibration| calibration.projected_ordinary_tax);
            ui.add_space(8.0);
            ui.columns(3, |columns| {
                compact_fact(
                    &mut columns[0],
                    "Full-year basis",
                    if totals.adjustment_basis_work_income > 0 {
                        format_sek(totals.adjustment_basis_work_income)
                    } else {
                        "Not selected".to_owned()
                    },
                );
                compact_fact(
                    &mut columns[1],
                    "Applied to payers",
                    if overridden_count > 0 {
                        format!("{shown_count} applied · {overridden_count} overridden")
                    } else {
                        format!(
                            "{shown_count} payer{}",
                            if shown_count == 1 { "" } else { "s" }
                        )
                    },
                );
                compact_fact(
                    &mut columns[2],
                    "Calibrated tax projection",
                    projected_tax
                        .map(format_sek)
                        .unwrap_or_else(|| "Needs a basis".to_owned()),
                );
            });
            ui.label(
                egui::RichText::new(format!(
                    "The {percent}% decision affects withholding only on rows marked as shown. A selected full-year basis also calibrates the annual tax projection."
                ))
                .small()
                .color(secondary_text()),
            );
        });
}

fn income_overview_table(
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

fn income_entry_name(entry: &IncomeEntry, index: usize) -> String {
    if entry.description.trim().is_empty() {
        format!("Income {}", index + 1)
    } else {
        entry.description.trim().to_owned()
    }
}

fn entry_period_text(entry: &IncomeEntry) -> String {
    format!(
        "{} {} – {} {}",
        month_name(entry.start.month),
        entry.start.day,
        month_name(entry.end.month),
        entry.end.day,
    )
}

fn optional_sek(value: u32) -> String {
    if value == 0 {
        "—".to_owned()
    } else {
        format_sek(value)
    }
}

fn income_eligibility_short(kind: IncomeKind) -> &'static str {
    match (kind.is_pgi_eligible(), kind.is_sgi_eligible()) {
        (true, true) => "PGI + SGI",
        (true, false) => "PGI",
        _ => "—",
    }
}

fn income_kind_short(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::AnnualSalary => "Annual salary",
        IncomeKind::MonthlySalary => "Monthly salary",
        IncomeKind::OneTimeSalary => "One-time salary",
        IncomeKind::MonthlyOccupationalPension => "Monthly tjänstepension",
        IncomeKind::AnnualOccupationalPension => "Annual tjänstepension",
        IncomeKind::OwnCompanyDividend => "Own-AB dividend",
    }
}

fn selected_income_impact(
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
                "Jämkning shown to payer",
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
            "Current-year pension salary basis",
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

fn audit_section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
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

fn audit_row(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
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

fn income_totals_footer(ui: &mut egui::Ui, plan: &IncomePlan, table: u8, age_group: TaxAgeGroup) {
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
                    "Salary {} · Tjänstepension received {} · Dividend {} · Employer pension contributions {}",
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

fn compact_fact(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.label(secondary_label(label));
        ui.label(egui::RichText::new(value).strong().color(primary_text()));
    });
}

fn card(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(surface_color())
        .stroke(egui::Stroke::new(1.0, border_color()))
        .corner_radius(8.0)
        .inner_margin(16.0)
        .show(ui, contents);
}

fn card_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
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

fn income_entry_editor(
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
                        date_editor(&mut columns[0], "From", entry.id, &mut entry.start);
                    period_changed |= date_editor(
                        &mut columns[1],
                        "Through",
                        entry.id + 1_000_000,
                        &mut entry.end,
                    );
                });
                ui.horizontal_wrapped(|ui| {
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
                        "Jämkning shown to this payer",
                    );
                } else {
                    entry.adjustment_applies = false;
                }

                let mut custom = entry.custom_withholding_percent.is_some();
                if ui.checkbox(&mut custom, "Custom withholding").changed() {
                    entry.set_custom_withholding_enabled(custom);
                }
                if let Some(percent) = &mut entry.custom_withholding_percent {
                    percentage_editor(ui, "custom-withholding", percent);
                }
                ui.label(eligibility_badge(income_eligibility(entry.kind)));
            }

            ui.add_space(8.0);
            let mut use_actual = entry.actual_withholding.is_some();
            if ui
                .checkbox(&mut use_actual, "Enter actual tax withheld")
                .changed()
            {
                entry.set_actual_withholding_enabled(use_actual);
            }
            if let Some(actual) = &mut entry.actual_withholding {
                ui.horizontal_wrapped(|ui| {
                    ui.label(secondary_label("Actual withheld for this income row"));
                    ui.add(
                        egui::DragValue::new(actual)
                            .range(0..=MAX_INCOME)
                            .suffix(" SEK")
                            .speed(100.0),
                    );
                });
                ui.label(
                    egui::RichText::new(
                        "This amount overrides table, jämkning, custom percentage, and the normal dividend assumption.",
                    )
                    .small()
                    .color(secondary_text()),
                );
            }
            if let Some(withholding) = withholding {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Withholding used: {} · {}",
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
                .checkbox(&mut use_override, "Use actual monthly contribution")
                .changed()
            {
                premium.monthly_override = use_override.then_some(benchmark);
            }
            if let Some(actual) = &mut premium.monthly_override {
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

fn salary_exchange_editor(
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
                "Include this payment in current-year pension salary basis",
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
                    value_row(
                        ui,
                        "Current-year pension salary before exchange",
                        format_sek(allowance.pension_salary_basis_before),
                    );
                    value_row(
                        ui,
                        "Current-year pension salary after exchange",
                        format_sek(allowance.pension_salary_basis_after),
                    );
                    value_row(
                        ui,
                        "Indicative employer deduction ceiling after exchange",
                        format_sek(allowance.ceiling),
                    );
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
                    value_row(
                        ui,
                        "Available before this salary exchange",
                        format_sek(allowance.available_contribution),
                    );
                    value_row(
                        ui,
                        "Total tjänstepension contributions",
                        format_sek(allowance.total_employer_pension_contributions),
                    );
                    value_row(
                        ui,
                        "Share of pension salary after exchange",
                        format!("{:.2}%", allowance.contribution_share_of_basis()),
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
            let taxable_payment = entry.total_annual_amount();
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
            let per_day = VacationCompensation::amount_per_day(entry.amount);
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
                let benchmark = vacation.additional_benchmark_pension_premium(entry.amount);
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
    match (kind.is_pgi_eligible(), kind.is_sgi_eligible()) {
        (true, true) => "PGI eligible · SGI estimate eligible",
        (true, false) => "PGI eligible · does not establish ongoing SGI",
        _ => "PGI — · SGI —",
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
                    ui.add_space(12.0);
                    adjustment_editor(ui, &mut self.income_plan, self.table, self.age_group);
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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_STATE_STORAGE_KEY, &self.persisted_state());
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_millis(500)
    }
}

fn validation_message(issue: Option<IncomePlanValidationIssue>) -> String {
    match issue {
        Some(IncomePlanValidationIssue::InvalidPaymentPeriod { .. }) => {
            "Check that each payment period ends after it starts.".to_owned()
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

fn summary_tiles(ui: &mut egui::Ui, summaries: &[Summary<'_>; 3]) {
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

fn summary_tile(
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
                    "Total tjänstepension contribution",
                    format!(
                        "{} · {:.2}% of pension salary after exchange",
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

fn calculation_trace(
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
            trace_heading(ui, "4", "Reconciliation");
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
                "Share of pension salary after exchange",
                format!(
                    "{} total contributions ÷ {} pension-salary basis after exchange × 100",
                    format_sek(calculation.employer_pension_contributions),
                    format_sek(calculation.pension_salary_basis),
                ),
                format!("{:.2}%", calculation.employer_pension_share_of_basis()),
            );
        });
}

fn trace_heading(ui: &mut egui::Ui, step: &str, title: &str) {
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

fn trace_line(ui: &mut egui::Ui, label: &str, equation: impl Into<String>, result: String) {
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

fn cash_income_equation(entry: &IncomeEntry) -> String {
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

fn withholding_equation(
    entry: &IncomeEntry,
    estimate: EntryWithholding,
    table: u8,
    annual_work_income: u32,
) -> String {
    match estimate.rule {
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
        AppliedWithholding::CustomPercent(percent) => {
            format!("{} × custom {percent}%", format_sek(estimate.gross))
        }
        AppliedWithholding::None => "No preliminary withholding for this income type".to_owned(),
    }
}

fn format_signed_sek(value: i64) -> String {
    if value < 0 {
        format!("−{}", format_sek(value.unsigned_abs() as u32))
    } else {
        format_sek(value as u32)
    }
}

fn format_delta_sek(value: i64) -> String {
    if value > 0 {
        format!("+{}", format_sek(value as u32))
    } else {
        format_signed_sek(value)
    }
}

fn income_basis_trace_value(estimate: IncomeBasisEstimate) -> String {
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

fn tax_balance_summary(balance: TaxBalance) -> String {
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

fn tax_balance_value(balance: TaxBalance) -> String {
    match balance {
        TaxBalance::Debt(amount) => format!("−{}", format_sek(amount)),
        TaxBalance::Refund(amount) => format!("+{}", format_sek(amount)),
        TaxBalance::Settled => format_sek(0),
    }
}

fn tax_balance_color(balance: TaxBalance) -> egui::Color32 {
    match balance {
        TaxBalance::Debt(_) => egui::Color32::from_rgb(176, 42, 42),
        TaxBalance::Refund(_) => green_color(),
        TaxBalance::Settled => primary_text(),
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
    fn newly_added_income_becomes_the_selected_row() {
        let mut app = TaxApp::default();
        let id = app.add_income_for_editing();

        assert_eq!(app.selected_income_entry, Some(id));
        assert_eq!(app.income_plan.entries.last().unwrap().id, id);
    }

    #[test]
    fn persisted_state_restores_the_complete_plan_and_settings() {
        let mut plan = IncomePlan::with_monthly_salary(93_000);
        plan.adjustment_percent = Some(33);
        let pension_id = plan.add_entry(IncomeKind::MonthlyOccupationalPension);
        plan.entries
            .iter_mut()
            .find(|entry| entry.id == pension_id)
            .unwrap()
            .amount = 27_500;
        let app = TaxApp::from_persisted_state(PersistedAppState::new(
            34,
            TaxAgeGroup::AtLeast66AtYearStart,
            plan.clone(),
        ));

        assert_eq!(app.table, 34);
        assert_eq!(app.age_group, TaxAgeGroup::AtLeast66AtYearStart);
        assert_eq!(app.income_plan, plan);
        assert_eq!(app.persisted_state().income_plan, plan);
    }

    #[test]
    fn monthly_table_reference_requires_one_uniform_full_year_salary() {
        let mut plan = IncomePlan::with_monthly_salary(93_000);
        assert!(plan.has_uniform_monthly_table_reference());

        plan.entries[0].end = Date2026::new(10, 18);
        assert!(!plan.has_uniform_monthly_table_reference());

        plan = IncomePlan::with_annual_salary(1_116_000);
        assert!(plan.has_uniform_monthly_table_reference());

        plan.add_entry(IncomeKind::AnnualOccupationalPension);
        assert!(!plan.has_uniform_monthly_table_reference());
    }

    #[test]
    fn formatting_groups_sek_without_locale_dependencies() {
        assert_eq!(format_sek(0), "0 SEK");
        assert_eq!(format_sek(1_234_567), "1 234 567 SEK");
        assert_eq!(
            tax_balance_summary(TaxBalance::Debt(2_400)),
            "Expected balance: −2 400 SEK · Tax debt"
        );
        assert_eq!(
            tax_balance_summary(TaxBalance::Refund(350)),
            "Expected balance: +350 SEK · Tax refund"
        );
    }
}
