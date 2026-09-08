# Swedish tax calculations for 2026

[![CI](https://github.com/qpernil/swedish-tax/actions/workflows/ci.yml/badge.svg)](https://github.com/qpernil/swedish-tax/actions/workflows/ci.yml)

This crate implements Skatteverket monthly tax tables 29 through 42 and the
annual preliminary-tax formulas from SKV 433, edition 36, for income year 2026.

The calculations use the same assumptions as the published tables and are not
an individualized final tax assessment.

## Command-line programs

```sh
cargo run --bin tax-annual -- 34 1 216000
cargo run --bin tax-monthly -- 34 1 18000
```

Arguments are tax table, column, and gross income in whole SEK.

## Desktop application

The native egui application includes a reopenable income calculator for
salary, one-time compensation, occupational pension, and dividends within the
owner-managed company's gränsbelopp. It calculates the withholding applied by main
and secondary payers (including percentage adjustment decisions), reconciles
withholding with annual final tax, and estimates progress toward the 2026 PGI
and SGI ceilings. Regular salary rows can estimate an ITP 1-equivalent employer
pension contribution, while one-time salary rows can model an editable salary exchange,
the employer uplift, the remaining pension allowance, and the taxable cash payment. The
35-percent allowance can optionally use a fixed, editable pensionable salary from the
preceding tax year instead of the current-year salary basis. An optional confirmed total
for pension and insurance costs before the exchange can replace the app's component
estimates in the allowance calculation.
Vacation compensation is pensionable by default in the ITP 1-style model and shows
its estimated additional employer contribution separately. The editable scenario default
is 5.4% of monthly salary per paid day, a common collective-agreement calculation, and the
percentage, pensionability, and actual contribution amounts remain editable for individually
agreed terms.
Recurring salary rows can also use their 12-month projection as the annual income
basis behind a percentage jämkning decision. The annual result then shows the
calibration explicitly while preserving the row's actual payment dates.
Partial first and last months divide by the calendar days in that month by default;
an optional row setting instead uses the annual daily rate (monthly amount × 12 / 365).
The app also estimates the 2027 3:12 gränsbelopp for qualified shares: the
ownership-adjusted basic amount, wage-based allowance using 2026 payroll, the
50-times-salary cap, acquisition-cost interest, and saved allowance. Salary
rows marked as sourced from the owner's company or group feed the owner's 2026
cash compensation and, in one-person-company mode, the complete payroll basis.
For a normal privately held Swedish AB, the preliminary 2027 view treats the
20-percent amount as personal tax declared on K10 in 2028 rather than tax
withheld by the company when the dividend is paid.
The desktop app restores the selected table, age group, jämkning settings, and
complete income plan between launches. If later pension changes make an existing
salary exchange exceed its allowance, the plan is marked invalid and reports the
new maximum instead of silently calculating from inconsistent inputs.

```sh
cargo run -p swedish-tax-gui
```

The GUI is a separate workspace package. A normal `cargo build` from the
repository root builds both the core/CLI package and the graphical application.

The GUI saves the current plan model without schema version markers or migrations.
If saved data cannot be read, it is preserved and saving is disabled for that session.

## Shared C interface and iOS build

The `swedish-tax-ios` workspace crate exposes the tax core through a typed C ABI.
Build ARM64 device and Apple Silicon simulator slices and package them as an
XCFramework with:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo xtask ios --release
```

The task first regenerates `ios-ffi/include/SwedishTaxFFI.h` from the public
`#[repr(C)]` Rust types and `extern "C"` functions using its pinned `cbindgen`
library dependency. The generated header is checked in for review and Swift
module import; `cargo test --workspace` fails if it drifts from the Rust ABI.

The default artifact is `target/ios/SwedishTaxCore.xcframework`. The iOS Xcode
project consumes that location through its persisted
`RUST_CORE_ARTIFACTS_DIR` build setting; `--output PATH` remains available for
other consumers and packaging workflows.

## WebAssembly C library

Compile the same C interface for the browser with:

```sh
rustup toolchain install 1.98.1 --component rust-src --target wasm32-unknown-emscripten
RUSTUP_TOOLCHAIN=1.98.1 cargo xtask wasm --release
```

The task regenerates the shared C header and produces
`target/wasm/libswedish_tax_ios.a` and `target/wasm/SwedishTaxFFI.h`. It builds a
static library for `wasm32-unknown-emscripten`; the consuming .NET application
links that library into its runtime with `dotnet publish`.

`--output DIRECTORY` selects the output directory. `--target-dir DIRECTORY`
selects Cargo's build cache, otherwise `CARGO_TARGET_DIR` or the workspace's
`target` directory is used. The default output is `wasm` under that build cache.
Relative paths resolve from the Rust workspace. Omit `--release` for a debug build.

The default compiler flags are the configuration verified with .NET SDK 10.0.301:
`-C panic=abort -C target-cpu=mvp -C target-feature=-bulk-memory-opt,-call-indirect-overlong`.
`--rustflags FLAGS` replaces that preset so a consumer can supply its pinned settings.
The task rebuilds Rust's standard library with `-Z build-std=std,panic_abort` and
scopes `RUSTC_BOOTSTRAP=1` to the target build. Rust warns that the LLVM feature
flags are unstable. Compiler changes require browser parity verification.

The same C ABI powers the
[Blazor calculator](https://github.com/qpernil/swedish-tax-aspnet).
Its build script verifies the provider pin, generated declarations and fixture
copies, invokes `cargo xtask wasm`, and supplies the library to the .NET linker.
C# calls generated P/Invoke declarations, with all 10 functions, 21 structures and result cleanup covered by
native/browser tests. The editor interface returns structured validation,
invalid-plan totals, row/monthly amounts, vacation/pension details, exchange allowance
previews and policy defaults. Both clients map these Rust results; the saved exchange
request remains unchanged when its preview is limited to the permitted maximum.
See [the editor API contract](docs/ffi-plan-support.md).
The browser build pins its compiler configuration and uses abort-on-panic
behavior. Its supported build and ownership contract are described in the
[consumer engine guide](https://github.com/qpernil/swedish-tax-aspnet/blob/master/docs/rust-engine.md).
The iOS artifact and build workflow use the XCFramework described above.

The C interface supports the current generated header only. Rebuild consumers and
the Rust library together when its types or exports change.

## Shared test fixtures

`tests/fixtures` contains 17 checked-in input plans and expected results generated
directly from the Rust core. They verify client mappings for ordinary and mixed income,
proration, vacation, pensions, salary exchange, dividends and invalid inputs.
Generate or check them with the native developer tool:

```sh
cargo xtask fixtures
cargo xtask fixtures --check
```

The Blazor repository checks in exact copies for its native and browser tests;
its native-engine build rejects any difference from the pinned provider. Its
production calculations call the shared C ABI through P/Invoke, with the Rust
library compiled to WebAssembly and linked into the .NET runtime. The fixture
JSON is test data, independent of that compilation target.
See [the shared fixture guide](docs/shared-fixtures.md) for generation, schema
validation and consumer synchronization.

## Sources

- [SKV 433 technical specification](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55c/1766385913260/teknisk-beskrivning-skv-433-2026-utgava-36.pdf)
- [Official monthly tables](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba5af/1765287119989/allmanna-tabeller-manad.txt)
- [2026 withholding on one-time payments](https://www.skatteverket.se/foretag/arbetsgivare/arbetsgivaravgifterochskatteavdrag/skatteavdrag/engangsbelopp.4.361dc8c15312eff6fd3225e.html)
- [Worked examples](https://www.skatteverket.se/download/18.1522bf3f19aea8075ba55f/1765284831853/bilaga-3-exempel-till-skv-433-2026.pdf)
- [2026 pensionable income (PGI)](https://www.skatteverket.se/privat/skatter/arbeteochinkomst/pensionsgrundandeinkomstpgi.4.4f3d00a710cc9ae1c9c80008300.html)
- [Sickness-benefit qualifying income (SGI)](https://www.forsakringskassan.se/privatperson/sjukpenninggrundande-inkomst-sgi)
- [2026 ITP 1-equivalent pension-contribution benchmark](https://collectum.se/avtal-och-faktura/faktura-och-premier/aktuella-premier-och-basbelopp)
- [ITP pensionable salary, including vacation compensation](https://collectum.se/administration/sa-rapporterar-du-ratt/pensionsmedforande-lon)
- [Common 5.4% vacation-compensation calculation](https://www.unionen.se/rad-och-stod/rakna-ut-din-semesterersattning)
- [Employer pension-cost deduction rules](https://www4.skatteverket.se/rattsligvagledning/edition/2026.3/339021.html)
- [Changed 3:12 rules for income year 2026](https://www.skatteverket.se/foretag/drivaforetag/foretagsformer/famansforetag/andradereglerfordelagareifamansforetaginforinkomstdeklarationen2027.4.4a54dc8b19aa6175a152359.html)
- [2026 income-base amount used by the preliminary 2027 estimate](https://www.skatteverket.se/privat/skatter/beloppochprocent/2026/beloppochprocent2026kortversion.4.1522bf3f19aea8075ba89.html)

## Contributing and license

Development instructions are in [CONTRIBUTING.md](CONTRIBUTING.md). This
project is available under the [MIT License](LICENSE).
