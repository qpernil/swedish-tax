#ifndef SWEDISH_TAX_FFI_H
#define SWEDISH_TAX_FFI_H

/* Generated from ios-ffi Rust sources by cbindgen. Do not edit manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

// Named status values documented by the generated C contract.
#define SWEDISH_TAX_STATUS_OK 0

#define SWEDISH_TAX_STATUS_INVALID_INPUT 1

#define SWEDISH_TAX_STATUS_INTERNAL_ERROR 2

// Named deduction kinds documented by the generated C contract.
#define SWEDISH_TAX_DEDUCTION_AMOUNT 0

#define SWEDISH_TAX_DEDUCTION_PERCENT 1

// Stable C representation of a monthly tax-table lookup.
//
// All fields are fixed-width integers to keep the ABI straightforward for
// Swift and other consumers. `value` is SEK when `kind` is amount, otherwise
// it is a whole percentage.
typedef struct SwedishTaxDeductionResult {
  uint32_t status;
  uint32_t kind;
  uint32_t value;
} SwedishTaxDeductionResult;

typedef struct SwedishTaxAnnualTaxResult {
  uint32_t status;
  uint32_t assessed_income;
  uint32_t basic_allowance;
  uint32_t taxable_income;
  uint32_t state_income_tax;
  uint32_t municipal_income_tax;
  uint32_t burial_and_religious_fee;
  uint32_t pension_fee;
  uint32_t pension_fee_credit;
  uint32_t work_income_credit;
  uint32_t sickness_compensation_credit;
  uint32_t earned_income_credit;
  uint32_t public_service_fee;
  uint32_t total;
} SwedishTaxAnnualTaxResult;

typedef struct SwedishTaxAdjustmentCalibration {
  uint32_t basis_income;
  uint32_t percent;
  uint32_t formula_tax_at_basis;
  uint32_t assumed_tax_at_basis;
  int64_t implied_tax_adjustment;
  uint32_t projected_ordinary_tax;
} SwedishTaxAdjustmentCalibration;

typedef struct SwedishTaxWithholdingEntry {
  uint64_t entry_id;
  uint32_t gross;
  uint32_t withheld;
  uint32_t regular_withheld;
  uint32_t supplemental_withheld;
  uint32_t additional_withheld;
  uint32_t rule_kind;
  uint32_t rule_column;
  uint32_t rule_percent;
} SwedishTaxWithholdingEntry;

typedef struct SwedishTaxIncomeBasis {
  uint32_t kind;
  uint32_t estimated_basis;
  uint32_t maximum_basis;
} SwedishTaxIncomeBasis;

typedef struct SwedishTaxCalculationResult {
  uint32_t status;
  uint32_t monthly_income;
  uint32_t annual_income;
  uint32_t ordinary_income;
  uint32_t work_income;
  uint32_t pension_income;
  uint32_t dividend_income;
  uint32_t sgi_annual_rate;
  uint32_t deduction_kind;
  uint32_t deduction_value;
  struct SwedishTaxAnnualTaxResult annual_tax;
  uint32_t has_adjustment_calibration;
  struct SwedishTaxAdjustmentCalibration adjustment_calibration;
  uint32_t ordinary_final_tax;
  uint32_t dividend_tax;
  uint32_t total_tax;
  uint32_t withholding_total;
  struct SwedishTaxWithholdingEntry *withholding_entries;
  size_t withholding_entries_count;
  size_t withholding_entries_capacity;
  uint32_t withheld_tax;
  uint32_t regular_pension_premiums;
  uint32_t vacation_pension_premiums;
  uint32_t salary_exchange_sacrifice;
  uint32_t salary_exchange_pension_contributions;
  uint32_t pension_salary_basis;
  uint32_t employer_pension_contributions;
  double marginal_rate;
  struct SwedishTaxIncomeBasis pension_progress;
  struct SwedishTaxIncomeBasis sgi_progress;
} SwedishTaxCalculationResult;

typedef struct SwedishTaxDate {
  uint32_t month;
  uint32_t day;
} SwedishTaxDate;

typedef struct SwedishTaxOptionalU32 {
  uint32_t is_some;
  uint32_t value;
} SwedishTaxOptionalU32;

typedef struct SwedishTaxVacationCompensation {
  uint32_t is_some;
  uint32_t annual_entitlement_days;
  uint32_t payout_days;
  uint32_t rate_basis_points;
  uint32_t included_in_pension_salary_basis;
  struct SwedishTaxOptionalU32 pension_premium_override;
} SwedishTaxVacationCompensation;

typedef struct SwedishTaxRegularPensionPremium {
  uint32_t is_some;
  struct SwedishTaxOptionalU32 monthly_override;
} SwedishTaxRegularPensionPremium;

typedef struct SwedishTaxSalaryExchange {
  uint32_t is_some;
  uint32_t sacrificed_salary;
  uint32_t employer_adds_uplift;
  uint32_t uplift_basis_points;
  struct SwedishTaxOptionalU32 previous_year_pension_salary_basis;
  struct SwedishTaxOptionalU32 pension_and_insurance_costs_before_exchange;
} SwedishTaxSalaryExchange;

typedef struct SwedishTaxIncomeEntry {
  uint64_t id;
  uint32_t kind;
  uint32_t amount;
  struct SwedishTaxDate start;
  struct SwedishTaxDate end;
  uint32_t use_annual_daily_rate_for_partial_months;
  uint32_t payer_role;
  uint32_t own_company_sourced;
  uint32_t adjustment_applies;
  uint32_t use_full_year_projection_as_adjustment_basis;
  struct SwedishTaxOptionalU32 additional_withholding_per_payment;
  struct SwedishTaxOptionalU32 actual_withholding;
  struct SwedishTaxVacationCompensation vacation_compensation;
  struct SwedishTaxRegularPensionPremium regular_pension_premium;
  struct SwedishTaxSalaryExchange salary_exchange;
  uint32_t included_in_pension_salary_basis;
} SwedishTaxIncomeEntry;

typedef struct SwedishTaxDividendAllowanceInputs {
  uint32_t one_person_company;
  uint32_t ownership_basis_points;
  uint32_t other_qualified_ownership_basis_points;
  uint32_t spouse_ownership_basis_points;
  uint32_t company_cash_payroll_2026;
  uint32_t highest_related_cash_salary_2026;
  uint32_t acquisition_cost;
  struct SwedishTaxOptionalU32 acquisition_cost_interest_basis_points;
  uint32_t saved_allowance;
} SwedishTaxDividendAllowanceInputs;

typedef struct SwedishTaxPlanRequest {
  uint32_t table;
  uint32_t age_group;
  const struct SwedishTaxIncomeEntry *entries;
  size_t entries_count;
  struct SwedishTaxOptionalU32 adjustment_percent;
  struct SwedishTaxDividendAllowanceInputs dividend_allowance;
} SwedishTaxPlanRequest;

typedef struct SwedishTaxDividendAllowanceResult {
  uint32_t status;
  uint32_t issue_kind;
  uint32_t basic_amount;
  uint32_t owner_cash_salary;
  uint32_t company_cash_payroll;
  uint32_t joint_wage_basis;
  uint32_t joint_wage_basis_after_deduction;
  uint32_t wage_allowance_before_cap;
  uint32_t wage_cap_salary;
  uint32_t wage_cap;
  uint32_t wage_allowance;
  uint32_t acquisition_cost_interest_basis;
  uint32_t acquisition_cost_interest;
  uint32_t saved_allowance;
  uint32_t total;
  uint32_t tax_at_twenty_percent;
  uint32_t net_after_twenty_percent_tax;
} SwedishTaxDividendAllowanceResult;

typedef struct SwedishTaxPlanTotals {
  uint32_t work_income;
  uint32_t pension_income;
  uint32_t dividend_income;
  uint32_t sgi_annual_rate;
  uint32_t adjustment_basis_work_income;
  uint32_t pension_salary_basis;
  uint32_t regular_pension_premiums;
  uint32_t vacation_pension_premiums;
  uint32_t salary_exchange_sacrifice;
  uint32_t salary_exchange_pension_contributions;
  uint32_t ordinary_income;
  uint32_t monthly_taxable_income;
  uint32_t gross_income;
  uint32_t total_employer_pension_contributions;
  double employer_pension_share_of_basis;
} SwedishTaxPlanTotals;

// January through December; non-monthly entries return zero in every month.
typedef struct SwedishTaxMonthlyAmounts {
  uint32_t january;
  uint32_t february;
  uint32_t march;
  uint32_t april;
  uint32_t may;
  uint32_t june;
  uint32_t july;
  uint32_t august;
  uint32_t september;
  uint32_t october;
  uint32_t november;
  uint32_t december;
} SwedishTaxMonthlyAmounts;

// Preview uses min(requested sacrifice, maximum_sacrifice); saved inputs and plan totals are unchanged.
typedef struct SwedishTaxExchangeAllowance {
  uint32_t ceiling;
  uint32_t pension_salary_basis_before;
  uint32_t pension_salary_basis_after;
  struct SwedishTaxOptionalU32 previous_year_pension_salary_basis;
  struct SwedishTaxOptionalU32 pension_and_insurance_costs_before_exchange;
  uint32_t pension_contributions_before;
  uint32_t regular_pension_premiums;
  uint32_t vacation_pension_premiums;
  uint32_t other_exchange_contributions;
  uint32_t selected_exchange_contribution;
  uint32_t total_employer_pension_contributions;
  uint32_t available_contribution;
  uint32_t maximum_sacrifice;
  double contribution_share_of_basis;
} SwedishTaxExchangeAllowance;

typedef struct SwedishTaxEntrySupport {
  uint32_t status;
  uint64_t entry_id;
  uint32_t annual_amount;
  uint32_t total_annual_amount;
  uint32_t withholding_payment_count;
  uint32_t requested_additional_withholding;
  uint32_t vacation_compensation_amount;
  uint32_t regular_pension_premium_amount;
  uint32_t vacation_pension_premium_amount;
  uint32_t salary_exchange_sacrifice;
  uint32_t salary_exchange_pension_contribution;
  uint32_t pension_salary_basis_amount;
  uint32_t full_year_adjustment_basis_amount;
  uint32_t is_valid;
  uint32_t pension_benchmark_monthly;
  uint32_t suggested_vacation_days;
  double vacation_amount_per_day;
  struct SwedishTaxMonthlyAmounts monthly_amounts;
  uint32_t has_allowance;
  struct SwedishTaxExchangeAllowance allowance;
} SwedishTaxEntrySupport;

// issue_kind: 0 none, 1 invalid payment period, 2 exchange exceeds allowance. ID/maximum are meaningful only for their issue.
typedef struct SwedishTaxPlanSupport {
  uint32_t status;
  uint32_t issue_kind;
  uint64_t issue_entry_id;
  uint32_t issue_maximum;
  struct SwedishTaxPlanTotals totals;
  uint32_t has_uniform_monthly_table_reference;
  uint32_t salary_column;
  uint32_t pension_column;
  struct SwedishTaxEntrySupport *entries;
  size_t entries_count;
  size_t entries_capacity;
} SwedishTaxPlanSupport;

// Policy defaults for new editor values. Existing saved values remain explicit inputs.
typedef struct SwedishTaxPlanningPolicy {
  uint32_t regular_pension_monthly_threshold;
  uint32_t default_vacation_rate_basis_points;
  uint32_t default_exchange_uplift_basis_points;
  uint32_t employer_pension_allowance_maximum;
  uint32_t acquisition_cost_threshold;
} SwedishTaxPlanningPolicy;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Looks up a 2026 monthly deduction using the existing Rust tax core.
//
// `column` uses the public one-based tax-table column numbers 1 through 6.
// No Rust panic is allowed to unwind across the C boundary.
struct SwedishTaxDeductionResult swedish_tax_monthly_deduction(uint32_t table,
                                                               uint32_t column,
                                                               uint32_t gross_monthly_income);

struct SwedishTaxAnnualTaxResult swedish_tax_annual_tax(uint32_t table,
                                                        uint32_t column,
                                                        uint32_t gross_yearly_income);

struct SwedishTaxAnnualTaxResult swedish_tax_annual_tax_for_income_profile(uint32_t table,
                                                                           uint32_t age_group,
                                                                           uint32_t work_income,
                                                                           uint32_t pension_income);

struct SwedishTaxCalculationResult swedish_tax_calculate_plan(const struct SwedishTaxPlanRequest *request);

struct SwedishTaxDividendAllowanceResult swedish_tax_dividend_allowance_for_plan(const struct SwedishTaxPlanRequest *request);

void swedish_tax_calculation_result_free(struct SwedishTaxCalculationResult result);

// Editor details remain available for invalid payment periods and excessive exchanges.
// status describes request decoding, independently of issue_kind. Totals retain requested
// inputs (entry sacrifice is bounded by its payment); allowance previews use the permitted
// maximum. Rows follow input order. IDs must be unique within a plan.
//
// Ownership: request and entries must be aligned, readable and immutable until return.
// Returned entries are Rust-owned; copy before freeing. Return the original result exactly
// once to swedish_tax_plan_support_free, including on error. Never alter pointer/count/
// capacity, free with another allocator, or use a returned pointer after free.
// Build consumers against the matching generated header and Rust library.
struct SwedishTaxPlanSupport swedish_tax_plan_support(const struct SwedishTaxPlanRequest *request);

// Returns the original support allocation to Rust exactly once. A zero/error result is safe.
void swedish_tax_plan_support_free(struct SwedishTaxPlanSupport result);

// Allocation-free entry preview for editors. No plan-level allowance is returned.
// A null or undecodable entry returns INVALID_INPUT. The input is borrowed until return.
struct SwedishTaxEntrySupport swedish_tax_entry_support(const struct SwedishTaxIncomeEntry *entry);

struct SwedishTaxPlanningPolicy swedish_tax_planning_policy(void);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* SWEDISH_TAX_FFI_H */
