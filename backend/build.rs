use std::{env, path::PathBuf, process::Command};

fn main() {
    // CARGO_MANIFEST_DIR points to crate dire
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("..");

    let frontend_path = workspace_root.join("frontend");

    let target = workspace_root
        .canonicalize()
        .unwrap()
        .join("target")
        .join("frontend");

    let status = Command::new("npm")
        .arg("install")
        .current_dir(&frontend_path)
        .status()
        .unwrap();

    if !status.success() {
        panic!("npm build failed")
    }

    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&frontend_path)
        .env("OUTPUT_DIR", &target)
        .status()
        .unwrap();

    if !status.success() {
        panic!("npm build failed")
    }

    // re-run if source changed
    println!(
        "cargo:rerun-if-changed={}/src",
        frontend_path.to_string_lossy()
    );
    // re-run if package got modified
    println!(
        "cargo:rerun-if-changed={}/package.json",
        frontend_path.to_string_lossy()
    );
}
