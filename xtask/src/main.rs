use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const DEVICE_TARGET: &str = "aarch64-apple-ios";
const SIMULATOR_TARGET: &str = "aarch64-apple-ios-sim";
const LIBRARY_NAME: &str = "libswedish_tax_ios.a";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        return Err(usage());
    };
    if command != "ios" {
        return Err(format!("unknown command {command:?}\n{}", usage()));
    }

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is inside the workspace");
    let mut release = false;
    let mut output = workspace
        .join("target")
        .join("ios")
        .join("SwedishTaxCore.xcframework");

    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--release") => release = true,
            Some("--output") => {
                let Some(path) = arguments.next() else {
                    return Err("--output requires a path".to_owned());
                };
                output = absolute_path(workspace, Path::new(&path));
            }
            Some("--help" | "-h") => {
                println!("{}", usage());
                return Ok(());
            }
            _ => return Err(format!("unknown argument {argument:?}\n{}", usage())),
        }
    }

    generate_c_header(workspace)?;

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    build(&cargo, workspace, DEVICE_TARGET, release)?;
    build(&cargo, workspace, SIMULATOR_TARGET, release)?;

    let profile = if release { "release" } else { "debug" };
    let device_library = workspace
        .join("target")
        .join(DEVICE_TARGET)
        .join(profile)
        .join(LIBRARY_NAME);
    let simulator_library = workspace
        .join("target")
        .join(SIMULATOR_TARGET)
        .join(profile)
        .join(LIBRARY_NAME);
    let headers = workspace.join("ios-ffi").join("include");

    if output.exists() {
        fs::remove_dir_all(&output)
            .map_err(|error| format!("failed to replace {}: {error}", output.display()))?;
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    run_command(
        Command::new("xcodebuild")
            .current_dir(workspace)
            .arg("-create-xcframework")
            .arg("-library")
            .arg(&device_library)
            .arg("-headers")
            .arg(&headers)
            .arg("-library")
            .arg(&simulator_library)
            .arg("-headers")
            .arg(&headers)
            .arg("-output")
            .arg(&output),
    )?;

    println!("created {}", output.display());
    Ok(())
}

fn generate_c_header(workspace: &Path) -> Result<(), String> {
    let output = rendered_c_header(workspace)?;
    let header = workspace
        .join("ios-ffi")
        .join("include")
        .join("SwedishTaxFFI.h");
    if fs::read(&header).ok().as_deref() != Some(output.as_slice()) {
        fs::write(&header, output)
            .map_err(|error| format!("failed to write {}: {error}", header.display()))?;
        println!("generated {}", header.display());
    }
    Ok(())
}

fn rendered_c_header(workspace: &Path) -> Result<Vec<u8>, String> {
    let config_path = workspace.join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path)
        .map_err(|error| format!("failed to load {}: {error}", config_path.display()))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(workspace.join("ios-ffi"))
        .with_config(config)
        .generate()
        .map_err(|error| format!("failed to generate the iOS C header: {error}"))?;
    let mut output = Vec::new();
    bindings.write(&mut output);
    Ok(output)
}

fn build(cargo: &OsString, workspace: &Path, target: &str, release: bool) -> Result<(), String> {
    let mut command = Command::new(cargo);
    command
        .current_dir(workspace)
        .arg("build")
        .arg("--package")
        .arg("swedish-tax-ios")
        .arg("--target")
        .arg(target);
    if release {
        command.arg("--release");
    }
    run_command(&mut command)
}

fn run_command(command: &mut Command) -> Result<(), String> {
    eprintln!("running {command:?}");
    let status = command
        .status()
        .map_err(|error| format!("failed to start {command:?}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} exited with {status}"))
    }
}

fn absolute_path(workspace: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        workspace.join(path)
    }
}

fn usage() -> String {
    "usage: cargo xtask ios [--release] [--output PATH]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_c_header_matches_the_rust_abi() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask is inside the workspace");
        let expected = rendered_c_header(workspace).expect("the C header should generate");
        let header = workspace
            .join("ios-ffi")
            .join("include")
            .join("SwedishTaxFFI.h");
        let actual = fs::read(&header).expect("the checked-in C header should exist");
        assert_eq!(
            actual,
            expected,
            "{} is stale; run `cargo xtask ios`",
            header.display()
        );
    }
}
