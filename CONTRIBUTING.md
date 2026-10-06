# Contributing

## Development setup

Rust 1.95.0 is the minimum supported version. The repository toolchain file
selects stable Rust and installs `rustfmt` and Clippy automatically. The verified
WebAssembly build uses Rust 1.98.1 as described below.

Run the desktop application with:

```sh
cargo run -p swedish-tax-gui
```

## Before submitting a change

Run the same checks as CI:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI also checks the minimum supported Rust toolchain:

```sh
rustup toolchain install 1.95.0
cargo +1.95.0 check --workspace --all-targets
```

Tax formulas and thresholds should cite an authoritative source in code or in
the README. Changes to the shared C ABI must regenerate and commit
`ios-ffi/include/SwedishTaxFFI.h` with `cargo xtask ios --release` or
`cargo xtask wasm --release`. The latter needs Rust 1.98.1 with `rust-src` and
the `wasm32-unknown-emscripten` target for the verified browser configuration.
Provider CI builds the WebAssembly C library directly with that toolchain.

Shared calculation or client result-schema changes must update the fixtures with
`cargo xtask fixtures` and pass `cargo xtask fixtures --check`. The fixture
generator runs natively; `cargo test --workspace` checks the complete fixture set.
For a shared C API change, commit the provider first. In the canonical
`swedish-tax-aspnet` checkout, update `native-engine.json`, regenerate P/Invoke
declarations, synchronize the reference fixtures and rebuild the native libraries.
The consumer build script invokes the provider's `cargo xtask wasm` with its pinned
compiler flags; .NET owns the final runtime link and application publication.
Rebuild the XCFramework consumed by `swedish-tax-ios` and run its simulator tests.
Review documentation in all three repositories so API ownership, supported build
workflows and remaining client responsibilities agree. See
[the shared editor contract](docs/ffi-plan-support.md) and the
[consumer build guide](https://github.com/qpernil/swedish-tax-aspnet/blob/master/docs/rust-engine.md).

The browser application and its GitHub Pages deployment use the shared C ABI
through P/Invoke. Pages publishes the artifact built and tested by the
consumer's CI; a provider-only push does not update the deployed calculator.
