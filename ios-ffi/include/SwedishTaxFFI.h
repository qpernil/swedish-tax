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
  uint32_t version;
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

uint32_t swedish_tax_contract_version(void);

struct SwedishTaxCalculationResult swedish_tax_calculate_plan(const struct SwedishTaxPlanRequest *request);

struct SwedishTaxDividendAllowanceResult swedish_tax_dividend_allowance_for_plan(const struct SwedishTaxPlanRequest *request);

void swedish_tax_calculation_result_free(struct SwedishTaxCalculationResult result);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* SWEDISH_TAX_FFI_H */
