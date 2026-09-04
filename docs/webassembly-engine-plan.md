# WebAssembly engine plan

Status: planned

The `swedish-tax` crate will remain the single authoritative implementation of
tax, withholding, income-basis, pension, salary-exchange, and dividend rules.
Browser clients will execute that core locally through a dedicated WebAssembly
adapter. This gives the ASP.NET application the same engine as the desktop and
iOS applications without sending financial inputs to the server.

## Target architecture

```text
                         swedish-tax core
                        /                 \
             native C ABI                 WebAssembly adapter
                   |                              |
          iOS XCFramework                JavaScript ES module
                   |                              |
               Swift UI                   Blazor WebAssembly UI
```

The existing core and `ios-ffi` packages keep their current responsibilities.
A new `web-wasm` workspace package owns the browser boundary only; it must not
contain independent tax formulas.

Proposed workspace layout:

```text
swedish-tax/
├── src/                 # Authoritative domain and calculation code
├── gui/                 # Native desktop consumer
├── ios-ffi/             # Versioned C ABI for iOS
├── web-wasm/            # Versioned wasm-bindgen browser adapter
└── xtask/                # Reproducible native and web artifact builds
```

## Browser contract

The browser API will be small and versioned independently from internal Rust
types:

- `contract_version()` reports the adapter contract version.
- `calculate_plan(request)` returns the complete plan calculation, withholding
  rows, income-basis progress, pension totals, and reconciliation values.
- `dividend_allowance(request)` returns the complete preliminary 2027
  allowance breakdown or a typed domain issue.
- `engine_version()` optionally reports the Rust package version for diagnostics.

Requests and responses use explicit adapter DTOs serialized with
`serde-wasm-bindgen`. Internal domain structs are mapped at the boundary rather
than exported directly. This keeps browser compatibility stable when the Rust
implementation is refactored.

Every request includes a contract version, tax table, age group, complete
income entries, adjustment settings, and dividend-allowance inputs. Successful
responses contain all Rust result fields needed by consumers. Failures use a
tagged error object with a stable kind and relevant values such as `entry_id`
or `maximum`; consumers must not parse human-readable Rust error strings.

## Package and build

The `web-wasm` package will:

- build as a `cdylib` for `wasm32-unknown-unknown`;
- depend on `swedish-tax`, `wasm-bindgen`, `serde`, and `serde-wasm-bindgen`;
- expose JavaScript-friendly values and avoid raw pointers or the iOS C ABI;
- compile without native GUI dependencies; and
- produce a `.wasm` binary plus its generated JavaScript loader.

`cargo xtask web --release` should build the target, run `wasm-bindgen`, and
place deterministic artifacts in a requested output directory. The ASP.NET
repository will copy those generated assets into its static web assets during
its build or synchronization step. Generated artifacts are not hand-edited.

The build must fail when the generated interface or checked contract fixtures
are stale. Release assets should use content hashes or another cache-busting
mechanism so a browser cannot combine an old binary with a new loader.

## Runtime behavior

The WebAssembly module loads once when the Blazor application starts. All
calculation requests execute in the browser. The ASP.NET host serves static
assets but receives no salary, pension, withholding, or dividend data.

The adapter is synchronous internally because Rust calculations are pure and
short-lived. Browser consumers may expose an asynchronous API because module
loading and JavaScript interop are asynchronous. Consumers are responsible for
discarding stale responses when edits occur faster than results are rendered.

## Verification

The Rust repository will add:

1. DTO mapping tests covering every request and result field.
2. WebAssembly contract tests for success and every typed validation issue.
3. Node or headless-browser tests that load the generated module.
4. Shared parity fixtures for simple and complete income plans.
5. CI jobs that install the WebAssembly target, build the adapter, verify
   generated artifacts, and run the contract tests.

The same parity fixtures will be consumed by iOS and ASP.NET tests. Rust unit
tests remain the source of truth for business-rule correctness; consumer tests
verify transport, serialization, and presentation.

## Delivery phases

1. Define the versioned DTO contract and representative parity fixtures.
2. Add `web-wasm`, its mapping layer, error envelope, and tests.
3. Extend `xtask` and CI to generate reproducible browser artifacts.
4. Integrate the artifacts into the ASP.NET application in comparison mode.
5. Make Rust WebAssembly the production engine after every fixture matches.
6. Remove the independent C# calculation implementation after the cutover is
   proven and rollback is no longer required.

## Acceptance criteria

The adapter is ready for production when:

- every current Rust plan input and result is represented by the browser contract;
- iOS and browser parity fixtures produce identical domain results;
- all calculations work without a network request after assets are loaded;
- invalid plans return stable typed issues;
- stale artifact and contract-version mismatches fail clearly; and
- the Rust, WebAssembly, .NET, and browser test suites are green in CI.

The corresponding consumer plan lives in the
[`swedish-tax-aspnet`](https://github.com/qpernil/swedish-tax-aspnet)
repository.
