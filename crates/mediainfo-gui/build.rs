use std::fs;
use std::path::Path;
use std::process::Command;

fn run_npm(args: &[&str], dir: &Path) -> std::io::Result<std::process::ExitStatus> {
    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "npm"])
            .args(args)
            .current_dir(dir)
            .status()
    } else {
        Command::new("npm").args(args).current_dir(dir).status()
    }
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);
    let dist_index = manifest_path.join("dist").join("index.html");

    // Watch source files
    println!("cargo:rerun-if-changed={}/src", manifest_dir);
    println!("cargo:rerun-if-changed={}/index.html", manifest_dir);
    println!("cargo:rerun-if-changed={}/package.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/vite.config.ts", manifest_dir);
    println!("cargo:rerun-if-changed={}/tauri.conf.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/capabilities", manifest_dir);
    println!("cargo:rerun-if-changed={}/dist", manifest_dir);

    // If dist/index.html is missing, try to build it with npm
    if !dist_index.exists() {
        let nm = manifest_path.join("node_modules");
        if !nm.exists() {
            eprintln!("[build.rs] node_modules missing, attempting npm install...");
            let _ = run_npm(&["install"], manifest_path);
        }

        eprintln!("[build.rs] Building frontend with npm run build...");
        let build_status = run_npm(&["run", "build"], manifest_path);

        // If npm was not found or failed, provide fallback index.html so tauri-build doesn't panic
        if build_status.is_err() || !dist_index.exists() {
            eprintln!(
                "[build.rs] Warning: npm build failed or npm not found. Creating placeholder dist/index.html"
            );
            let dist_dir = manifest_path.join("dist");
            let _ = fs::create_dir_all(&dist_dir);
            let fallback_html = r#"<!DOCTYPE html><html><head><title>VuIO Media Info</title></head><body><div id="app">Loading...</div></body></html>"#;
            let _ = fs::write(&dist_index, fallback_html);
        }
    }

    // Run tauri build to embed dist/ and generate context
    tauri_build::build();
}
