// This is a small tool for making sure that we keep versions between all crates
// in the workspace consistent. It just panics if it finds an error and is
// supposed to be run as part of CI.

use glob::glob;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() {
    eprint!(
        "Checking Cargo workspace \"{}\" for crate version consistency ... ",
        Path::new(".").canonicalize().unwrap().display()
    );

    let workspace_cargo_toml_txt = std::fs::read_to_string("Cargo.toml").unwrap();

    if !workspace_cargo_toml_txt.trim().starts_with("[workspace]") {
        panic!(
            "Could not find workspace Cargo.toml at {}.\n\
                This tool has to be executed in the top-level directory \
                of the Cargo workspace.",
            Path::new("Cargo.toml").canonicalize().unwrap().display()
        );
    }

    let mut versions: BTreeMap<PathBuf, String> = BTreeMap::new();

    let version_regex = Regex::new("version\\s*=\\s*\"(\\d+\\.\\d+\\.\\d+)\"").unwrap();

    for entry in glob("./*/Cargo.toml").expect("Failed to read glob pattern") {
        let cargo_toml_path = entry.unwrap();
        let cargo_toml_txt = std::fs::read_to_string(&cargo_toml_path).unwrap();

        for line in cargo_toml_txt.lines() {
            if let Some(caps) = version_regex.captures(line) {
                let version = caps[1].to_string();
                versions.insert(cargo_toml_path.clone(), version);
                break;
            }
        }

        if !versions.contains_key(&cargo_toml_path) {
            panic!(
                "Could not find `version` field in {}",
                cargo_toml_path.display()
            );
        }
    }

    let reference_version = versions.values().next().unwrap();

    if !versions.values().all(|v| v == reference_version) {
        eprintln!("Crate versions found:");
        for (cargo_toml_path, version) in &versions {
            eprintln!("  {} = {}", cargo_toml_path.display(), version);
        }

        panic!("Not all crate versions are the same, please keep them in sync!");
    }

    eprintln!("check passed");
}
