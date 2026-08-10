# Contributing

## Development setup

Install the stable Rust toolchain. The repository toolchain file also installs
`rustfmt` and Clippy automatically.

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

Tax formulas and thresholds should cite an authoritative source in code or in
the README. Changes to the iOS ABI must regenerate and commit
`ios-ffi/include/SwedishTaxFFI.h` with `cargo xtask ios --release`.
