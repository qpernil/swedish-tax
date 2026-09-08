# Shared C editor support

The generated C header is the layout and ownership authority. The interface supports
one current contract, with no version query, request marker or versioned export names.
Build each client against the same header and Rust library; interface changes require
rebuilding consumers together with the provider.

- `swedish_tax_plan_support` borrows a plan request and returns totals,
  input-ordered row details, an optional exchange allowance per row, uniform-monthly
  reference eligibility and age-dependent salary/pension columns.
- `swedish_tax_plan_support_free` consumes the original result exactly once.
  Copy any rows first. Pointer, length and capacity must remain intact. Rust owns
  the allocation; a zero/error result can also be freed. No pointer survives cleanup.
- `swedish_tax_entry_support` returns an allocation-free row preview for standalone
  editor controls. Its allowance is absent because an allowance requires the whole plan.
- `swedish_tax_planning_policy` returns defaults and explanatory thresholds from
  the Rust core. Defaults apply to new inputs, preserving previously saved values.

## Validation and previews

Status describes decoding or integration failure. A decoded plan returns status OK
with issue kind 0 (none), 1 (invalid payment period) or 2 (exchange exceeds allowance).
The issue carries the original 64-bit row ID and, for exchange issues, the maximum.
Invalid periods take precedence, followed by the first excessive saved exchange.
IDs must be unique. Null requests, unsupported enums and duplicate IDs are
invalid input. An empty plan has zero totals and an empty buffer. The core's date
clamping remains in effect; the browser's stricter input-schema checks remain separate.

Plan and row totals retain the requested inputs. Entry sacrifice is bounded by its
payment amount, while validation checks the saved request. An allowance preview uses
min(requested sacrifice, maximum sacrifice) for contribution, remaining salary basis
and ceiling. It does not mutate saved inputs or make the invalid plan valid. Previous-year
bases and confirmed costs retain their core semantics. Invalid plans have editor details
but no annual projection or partial withholding through the calculation ABI.

The implementation delegates arithmetic to existing Rust core methods, including
rounding and saturation ordering. Monthly fields run January through December and are
zero for nonmonthly entries. Benchmark is zero where unavailable. Vacation suggestions
use the supplied entitlement, or 25 days when vacation is absent; daily value is zero
when vacation is absent. A vacation daily value describes its explicit input even if
the entry kind does not apply a vacation payout.

## Consumers and verification

Swift uses allocation-free row previews and copied plan support; C# copies plan support
inside `try/finally`. Both clients retain input models, persistence, enum/ID mapping,
formatting and arithmetic over final tax results. They do not reproduce proration,
pension benchmarks, exchange binary search or plan validation. The C# model constructor
constants initialize new model values; interactive creation uses Rust policy.

Provider tests cover all income kinds, both proration methods, extreme values, invalid
plans, prior-year bases, confirmed costs, validation precedence, null/empty input and
allocation cleanup. [Shared JSON reference fixtures](shared-fixtures.md) generated
by the native `cargo xtask fixtures` tool include excessive requests above the
payment and saturation. Consumer tests verify mappings and native C layouts, with the
browser harness checking the same fields against native-generated reference results.
The browser links this crate into .NET WebAssembly with panic=abort: unexpected Rust
panics terminate execution and are not recoverable errors.
