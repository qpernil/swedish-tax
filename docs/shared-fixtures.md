# Shared calculation fixtures

`tests/fixtures/*.json` contains 17 checked-in reference scenarios generated directly
from the Rust core. These are the authoritative expected results for client mapping
tests; the .NET repository checks in matching copies. The native `xtask` binary owns generation and verification; it is developer
tooling and is not linked into either client. Its private `fixtures` module holds the
scenario inputs, fixture-schema decoding and result serialization.

## Generate and verify

```sh
cargo xtask fixtures
cargo xtask fixtures --check
cargo test -p xtask
```

Generation writes or replaces the expected fixtures under `tests/fixtures`. When
removing a scenario, delete its obsolete JSON file as part of the same change.
Check mode reads the directory without modifying files and compares the complete
file contents and the set of JSON filenames. Workspace tests include the same check, plus fixture-schema validation and
coverage of calculation/editor fields. The tool needs only the host Rust toolchain.

Each fixture contains `request` and `response`. The request describes the table,
age group and complete income plan. The response records the Rust calculation,
plan/row totals, monthly values, withholding, pension and exchange previews,
income bases, dividend allowance and typed validation issues. All monetary results
come from existing Rust methods; the tool serializes them without implementing tax
rules. Explicit input DTOs preserve the reference schema and reject unknown fields.

The fixtures describe the current input and result shapes without version markers.
JSON preserves full 64-bit identifiers when read by Rust and .NET. Saved workspaces
have their own schema and contain inputs, not these derived reference responses.

## Coverage and interpretation

The scenarios cover ordinary and mixed income, both ages, both partial-month methods,
configurable vacation rates, current and previous-year pension bases, confirmed costs,
actual and additional withholding, dividends, invalid plans, saturation and large IDs.
Sixteen requests can be passed to the typed C client. One fixture exercises only the
JSON fixture/DTO schema by supplying an unknown input field.

Invalid periods and excessive exchanges retain editor details but suppress the annual
projection. The reference can record independently calculated partial withholding;
the C calculation API returns no partial withholding for invalid plans. Client parity
tests explicitly account for that difference. Business validation and preview semantics
are defined in [the shared editor contract](ffi-plan-support.md).

## Consumer synchronization

After an intentional change, regenerate and review the fixtures, then commit the Rust
provider. Update the Blazor repository's `native-engine.json` and copy this complete
`tests/fixtures` directory into its `tests/fixtures`. Commit those copies with the
consumer pin and mapping changes. The consumer build rejects missing,
extra or different files and checks its generated C declarations against the provider.

The .NET suite compares every returned field with these reference results. Browser
application tests also use selected scenarios. The separate `tests/RustPInvoke` harness
in the consumer generates its own low-level native C reference cases to verify all
exports, C layouts, P/Invoke mappings and Rust allocation cleanup in WebAssembly.

Production consumers use one shared C ABI: Swift calls it directly and Blazor uses
P/Invoke. Blazor compiles the Rust C library for `wasm32-unknown-emscripten` and links
it into `dotnet.native.wasm`. The native fixture generator is independent of this
browser build and exposes no application API or JavaScript module.

See the [consumer engine guide](https://github.com/qpernil/swedish-tax-aspnet/blob/master/docs/rust-engine.md)
for its pinned toolchain, build and browser verification commands.
