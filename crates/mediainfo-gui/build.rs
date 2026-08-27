use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);

    // Watch source files
    println!("cargo:rerun-if-changed={}/src", manifest_dir);
    println!("cargo:rerun-if-changed={}/index.html", manifest_dir);
    println!("cargo:rerun-if-changed={}/package.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/vite.config.ts", manifest_dir);
    println!("cargo:rerun-if-changed={}/tauri.conf.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/capabilities", manifest_dir);
    println!("cargo:rerun-if-changed={}/dist", manifest_dir);

    // Ensure node_modules exists
    let nm = manifest_path.join("node_modules");
    if !nm.exists() {
        eprintln!("[build.rs] node_modules missing, running npm install...");
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(&manifest_dir)
            .status()
            .expect("failed to run npm install");
        assert!(status.success(), "npm install failed");
    }

    // Always run npm build so dist/ is guaranteed fresh before tauri-build runs
    eprintln!("[build.rs] Building frontend with npm run build...");
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to run npm run build");
    assert!(status.success(), "npm run build failed");

    // Run tauri build to embed dist/ and generate context
    tauri_build::build();
}
