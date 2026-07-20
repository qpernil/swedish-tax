use eframe::egui;
use swedish_tax::{
    AnnualTax, MAX_TAX_TABLE, MIN_TAX_TABLE, TaxColumn, TaxDeduction, annual_tax, marginal_rate,
    monthly_deduction,
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
enum IncomePeriod {
    Monthly,
    Annual,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Calculation {
    monthly_income: u32,
    annual_income: u32,
    table_deduction: TaxDeduction,
    annual_tax: AnnualTax,
    marginal_rate: f64,
}

impl Calculation {
    fn new(table: u8, column: TaxColumn, period: IncomePeriod, income: u32) -> Option<Self> {
        let (monthly_income, annual_income) = match period {
            IncomePeriod::Monthly => (income, income.saturating_mul(12)),
            IncomePeriod::Annual => (income / 12, income),
        };
        let table_deduction = monthly_deduction(table, column, monthly_income)?;
        let annual_tax = annual_tax(table, column, annual_income)?;
        let marginal_rate = marginal_rate(table, column, monthly_income)?;

        Some(Self {
            monthly_income,
            annual_income,
            table_deduction,
            annual_tax,
            marginal_rate,
        })
    }

    const fn formula_monthly_tax(self) -> u32 {
        self.annual_tax.total / 12
    }

    const fn formula_monthly_net(self) -> u32 {
        self.monthly_income
            .saturating_sub(self.formula_monthly_tax())
    }

    fn effective_rate(self) -> f64 {
        if self.annual_income == 0 {
            0.0
        } else {
            f64::from(self.annual_tax.total) * 100.0 / f64::from(self.annual_income)
        }
    }
}

struct TaxApp {
    table: u8,
    column: u8,
    period: IncomePeriod,
    income: u32,
}

impl Default for TaxApp {
    fn default() -> Self {
        Self {
            table: 32,
            column: 1,
            period: IncomePeriod::Monthly,
            income: DEFAULT_MONTHLY_INCOME,
        }
    }
}

impl TaxApp {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&context.egui_ctx);
        Self::default()
    }

    fn selected_column(&self) -> TaxColumn {
        match self.column {
            1 => TaxColumn::Column1,
            2 => TaxColumn::Column2,
            3 => TaxColumn::Column3,
            4 => TaxColumn::Column4,
            5 => TaxColumn::Column5,
            6 => TaxColumn::Column6,
            _ => unreachable!("the column selector only exposes columns 1 through 6"),
        }
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
                    control_group(ui, "Income basis", |ui| {
                        ui.selectable_value(&mut self.period, IncomePeriod::Monthly, "Monthly");
                        ui.selectable_value(&mut self.period, IncomePeriod::Annual, "Annual");
                    });

                    ui.add_space(12.0);
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
                        ui.label(secondary_label("Column"));
                        let response = egui::ComboBox::from_id_salt("tax-column")
                            .selected_text(self.column.to_string())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for column in 1..=6 {
                                    ui.selectable_value(
                                        &mut self.column,
                                        column,
                                        format!("Column {column}"),
                                    );
                                }
                            })
                            .response;
                        response.on_hover_ui(column_selector_help);
                    });

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        let label = match self.period {
                            IncomePeriod::Monthly => "Monthly income",
                            IncomePeriod::Annual => "Annual income",
                        };
                        ui.label(secondary_label(label));
                        ui.add_sized(
                            [180.0, 28.0],
                            egui::DragValue::new(&mut self.income)
                                .range(0..=MAX_INCOME)
                                .speed(1_000.0)
                                .suffix(" SEK"),
                        );
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
                    egui::RichText::new(format!("Table {} / Column {}", self.table, self.column))
                        .color(blue_color()),
                );
            });
        });
        ui.add_space(10.0);

        let summaries = [
            (
                "Official table",
                table_deduction_text(calculation.table_deduction),
                None,
                blue_color(),
                None,
            ),
            (
                "Annual formula",
                format_sek(calculation.annual_tax.total),
                Some(format!("Marginal tax: {:.1}%", calculation.marginal_rate)),
                green_color(),
                Some(marginal_rate_help as HoverHelp),
            ),
            (
                "Monthly net",
                format_sek(calculation.formula_monthly_net()),
                None,
                primary_text(),
                None,
            ),
        ];
        summary_tiles(ui, &summaries);

        ui.add_space(24.0);
        comparison(ui, calculation);

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(18.0);
        ui.label(
            egui::RichText::new("Annual formula breakdown")
                .strong()
                .size(17.0)
                .color(primary_text()),
        );
        ui.add_space(8.0);
        annual_breakdown(ui, calculation.annual_tax);
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

fn column_selector_help(ui: &mut egui::Ui) {
    const COLUMNS: [&str; 6] = [
        "1. Salary, under 66. Work income eligible for the earned-income tax credit.",
        "2. Pension, 66 or older. No general pension contribution or earned-income tax credit.",
        "3. Salary, 66 or older. Work income eligible for the enhanced earned-income tax credit.",
        "4. Sickness or activity compensation, under 66. Eligible for its specific tax reduction.",
        "5. Other pensionable compensation. For example unemployment benefits; no earned-income tax credit.",
        "6. Pension, under 66. No general pension contribution or earned-income tax credit.",
    ];

    ui.set_max_width(420.0);
    ui.label(egui::RichText::new("2026 table columns").strong());
    ui.add_space(4.0);
    for description in COLUMNS {
        ui.label(description);
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Age is determined at the beginning of the income year.")
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
                    if let Some(calculation) = Calculation::new(
                        self.table,
                        self.selected_column(),
                        self.period,
                        self.income,
                    ) {
                        self.results(ui, calculation);
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

fn control_group(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.label(secondary_label(label));
        ui.horizontal(contents);
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

fn comparison(ui: &mut egui::Ui, calculation: Calculation) {
    ui.label(
        egui::RichText::new("Table and formula")
            .strong()
            .size(17.0)
            .color(primary_text()),
    );
    ui.add_space(8.0);
    egui::Grid::new("comparison-grid")
        .num_columns(2)
        .striped(true)
        .min_col_width(220.0)
        .show(ui, |ui| {
            value_row(
                ui,
                "Monthly gross income",
                format_sek(calculation.monthly_income),
            );
            value_row(
                ui,
                "Annual gross income",
                format_sek(calculation.annual_income),
            );
            value_row(
                ui,
                "Official table deduction",
                table_deduction_text(calculation.table_deduction),
            );
            value_row(
                ui,
                "Formula tax per month",
                format_sek(calculation.formula_monthly_tax()),
            );
            value_row(
                ui,
                "Formula effective rate",
                format!("{:.2}%", calculation.effective_rate()),
            );
            match calculation.table_deduction {
                TaxDeduction::Amount(amount) => {
                    let difference =
                        i64::from(calculation.formula_monthly_tax()) - i64::from(amount);
                    value_row(ui, "Formula minus table", format_signed_sek(difference));
                }
                TaxDeduction::Percent(percent) => {
                    let difference = calculation.effective_rate() - f64::from(percent);
                    value_row(ui, "Formula minus table", format!("{difference:+.2} pp"));
                }
            }
        });
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

fn format_credit(value: u32) -> String {
    if value == 0 {
        format_sek(0)
    } else {
        format!("-{}", format_sek(value))
    }
}

fn format_signed_sek(value: i64) -> String {
    match value.cmp(&0) {
        std::cmp::Ordering::Greater => format!("+{}", format_sek(value as u32)),
        std::cmp::Ordering::Less => format!("-{}", format_sek(value.unsigned_abs() as u32)),
        std::cmp::Ordering::Equal => format_sek(0),
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
    fn default_income_is_the_highest_whole_monthly_income_below_the_state_tax_breakpoint() {
        let app = TaxApp::default();
        let calculation =
            Calculation::new(app.table, app.selected_column(), app.period, app.income).unwrap();

        assert_eq!(app.income, 55_033);
        assert_eq!(calculation.annual_tax.state_income_tax, 0);
        assert!(app.income * 12 <= 660_400);
        assert!((app.income + 1) * 12 > 660_400);
    }

    #[test]
    fn monthly_input_uses_the_exact_monthly_lookup_and_annualizes_income() {
        let calculation =
            Calculation::new(34, TaxColumn::Column1, IncomePeriod::Monthly, 18_000).unwrap();

        assert_eq!(calculation.monthly_income, 18_000);
        assert_eq!(calculation.annual_income, 216_000);
        assert_eq!(
            calculation.table_deduction,
            monthly_deduction(34, TaxColumn::Column1, 18_000).unwrap()
        );
        assert_eq!(
            calculation.annual_tax,
            annual_tax(34, TaxColumn::Column1, 216_000).unwrap()
        );
        assert_eq!(
            calculation.formula_monthly_tax(),
            calculation.annual_tax.total / 12
        );
    }

    #[test]
    fn annual_input_uses_one_twelfth_for_the_table_lookup() {
        let calculation =
            Calculation::new(32, TaxColumn::Column3, IncomePeriod::Annual, 420_011).unwrap();

        assert_eq!(calculation.monthly_income, 35_000);
        assert_eq!(calculation.annual_income, 420_011);
        assert_eq!(
            calculation.table_deduction,
            monthly_deduction(32, TaxColumn::Column3, 35_000).unwrap()
        );
        assert_eq!(
            calculation.annual_tax,
            annual_tax(32, TaxColumn::Column3, 420_011).unwrap()
        );
    }

    #[test]
    fn marginal_rate_uses_annual_formula() {
        let calculation =
            Calculation::new(34, TaxColumn::Column1, IncomePeriod::Annual, 216_000).unwrap();

        let expected = f64::from(38_894 - 35_889) * 100.0 / 12_000.0;
        assert_eq!(calculation.marginal_rate, expected);
    }

    #[test]
    fn zero_income_has_zero_tax_and_a_stable_rate() {
        let calculation =
            Calculation::new(33, TaxColumn::Column1, IncomePeriod::Monthly, 0).unwrap();

        assert_eq!(calculation.table_deduction, TaxDeduction::Amount(0));
        assert_eq!(calculation.annual_tax.total, 0);
        assert_eq!(calculation.formula_monthly_net(), 0);
        assert_eq!(calculation.effective_rate(), 0.0);
    }

    #[test]
    fn every_published_table_is_available() {
        for table in MIN_TAX_TABLE..=MAX_TAX_TABLE {
            assert!(
                Calculation::new(table, TaxColumn::Column1, IncomePeriod::Monthly, 35_000)
                    .is_some(),
                "table {table}"
            );
        }
    }

    #[test]
    fn formatting_groups_sek_without_locale_dependencies() {
        assert_eq!(format_sek(0), "0 SEK");
        assert_eq!(format_sek(1_234_567), "1 234 567 SEK");
        assert_eq!(format_signed_sek(-2_400), "-2 400 SEK");
        assert_eq!(format_signed_sek(350), "+350 SEK");
    }
}
