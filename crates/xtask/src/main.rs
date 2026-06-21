#![forbid(unsafe_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use calculator_core::ProtocolVersion;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("generate-types") => generate_types(),
        Some("check-generated") => check_generated(),
        Some("check-protocol-compatibility") => check_protocol_compatibility(),
        Some("check-no-floats") => check_no_floats(),
        Some(command) => Err(format!("unknown xtask command: {command}")),
        None => Err(String::from(
            "usage: cargo xtask <generate-types|check-generated|check-protocol-compatibility|check-no-floats>",
        )),
    }
}

fn generate_types() -> Result<(), String> {
    let path = generated_dto_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, generated_dto_contents()).map_err(|error| error.to_string())?;
    Ok(())
}

fn check_generated() -> Result<(), String> {
    let path = generated_dto_path();
    let actual = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let expected = generated_dto_contents();
    if actual != expected {
        return Err(format!(
            "{} is stale; run `cargo xtask generate-types`",
            path.display()
        ));
    }
    Ok(())
}

fn check_protocol_compatibility() -> Result<(), String> {
    let version = ProtocolVersion::CURRENT;
    let path = protocol_snapshot_path(version);
    let expected = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let actual = generated_dto_contents();
    if actual != expected {
        return Err(format!(
            "protocol DTO surface no longer matches {}; update ProtocolVersion and add the matching snapshot when the public DTO contract changes",
            path.display()
        ));
    }
    Ok(())
}

fn check_no_floats() -> Result<(), String> {
    let root = Path::new("crates").join("calculator-core").join("src");
    let mut violations = Vec::new();
    visit_rs_files(&root, &mut |path| {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        for (index, line) in text.lines().enumerate() {
            if line.contains("f32") || line.contains("f64") {
                violations.push(format!("{}:{}", path.display(), index + 1));
            }
        }
        Ok(())
    })?;

    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "calculator-core must not use f32/f64:\n{}",
            violations.join("\n")
        ))
    }
}

fn visit_rs_files(
    path: &Path,
    callback: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, callback)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            callback(&path)?;
        }
    }
    Ok(())
}

fn generated_dto_path() -> PathBuf {
    Path::new("packages")
        .join("calculator")
        .join("src")
        .join("generated")
        .join("dto.ts")
}

fn protocol_snapshot_path(version: ProtocolVersion) -> PathBuf {
    Path::new("crates")
        .join("xtask")
        .join("snapshots")
        .join(format!(
            "protocol-{}.{}.dto.ts",
            version.major, version.minor
        ))
}

fn generated_dto_contents() -> &'static str {
    include_str!("../templates/dto.ts")
}
