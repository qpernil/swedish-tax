//! Compile the shared C interface for linking into the .NET WebAssembly runtime.

use std::{env, ffi::OsString, fs, path::Path, process::Command};

use crate::{LIBRARY_NAME, absolute_path, generate_c_header, run_command};

const TARGET: &str = "wasm32-unknown-emscripten";
// The .NET 10.0.301 optimizer cannot read the newer LLVM feature labels.
const RUSTFLAGS: &str =
    "-C panic=abort -C target-cpu=mvp -C target-feature=-bulk-memory-opt,-call-indirect-overlong";

pub(super) fn run(
    workspace: &Path,
    mut arguments: impl Iterator<Item = OsString>,
) -> Result<(), String> {
    let mut release = false;
    let mut output = None;
    let mut target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(|path| absolute_path(workspace, Path::new(&path)))
        .unwrap_or_else(|| workspace.join("target"));
    let mut rustflags = OsString::from(RUSTFLAGS);
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--release") => release = true,
            Some("--output" | "--target-dir" | "--rustflags") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("{} requires a value", argument.to_string_lossy()))?;
                match argument.to_str() {
                    Some("--output") => output = Some(absolute_path(workspace, Path::new(&value))),
                    Some("--target-dir") => {
                        target_dir = absolute_path(workspace, Path::new(&value))
                    }
                    _ => rustflags = value,
                }
            }
            Some("--help" | "-h") => {
                println!(
                    "usage: cargo xtask wasm [--release] [--output DIRECTORY] [--target-dir DIRECTORY] [--rustflags FLAGS]\n\n\
                     Builds {TARGET} with the selected Rust toolchain and rust-src.\n\
                     Outputs {LIBRARY_NAME} and SwedishTaxFFI.h (default: wasm under the Cargo artifact directory).\n\
                     Cargo artifacts use --target-dir, CARGO_TARGET_DIR, or target, in that order.\n\
                     Relative paths are resolved from the Rust workspace.\n\
                     Default Rust flags: {RUSTFLAGS}"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown wasm argument {argument:?}; use --help")),
        }
    }
    let output = output.unwrap_or_else(|| target_dir.join("wasm"));
    generate_c_header(workspace)?;

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command
        .current_dir(workspace)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("RUSTFLAGS", rustflags)
        // Keep this override scoped to the target build, including its standard library.
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["build", "--package", "swedish-tax-ios", "--target", TARGET])
        .arg("--target-dir")
        .arg(&target_dir)
        .args(["--locked", "-Z", "build-std=std,panic_abort"]);
    if release {
        command.arg("--release");
    }
    run_command(&mut command)?;

    fs::create_dir_all(&output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let library = target_dir
        .join(TARGET)
        .join(if release { "release" } else { "debug" })
        .join(LIBRARY_NAME);
    for (source, name) in [
        (library, LIBRARY_NAME),
        (
            workspace.join("ios-ffi/include/SwedishTaxFFI.h"),
            "SwedishTaxFFI.h",
        ),
    ] {
        let destination = output.join(name);
        if source != destination {
            fs::copy(&source, &destination).map_err(|error| {
                format!(
                    "failed to copy {} to {}: {error}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }
    println!(
        "created {} and SwedishTaxFFI.h in {}",
        LIBRARY_NAME,
        output.display()
    );
    Ok(())
}
