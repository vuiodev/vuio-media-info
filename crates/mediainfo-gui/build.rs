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

    // Watch frontend source files and config
    println!("cargo:rerun-if-changed={}/src", manifest_dir);
    println!("cargo:rerun-if-changed={}/index.html", manifest_dir);
    println!("cargo:rerun-if-changed={}/package.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/vite.config.ts", manifest_dir);
    println!("cargo:rerun-if-changed={}/tsconfig.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/tauri.conf.json", manifest_dir);
    println!("cargo:rerun-if-changed={}/capabilities", manifest_dir);

    // Build fresh frontend assets on compile if node_modules or npm exists
    let nm = manifest_path.join("node_modules");
    if !nm.exists() {
        eprintln!("[build.rs] node_modules missing, attempting npm install...");
        let _ = run_npm(&["install"], manifest_path);
    }

    eprintln!("[build.rs] Building fresh frontend with npm run build...");
    let _build_status = run_npm(&["run", "build"], manifest_path);

    // If npm build failed and dist/index.html still missing, provide fallback so tauri-build doesn't panic
    if !dist_index.exists() {
        eprintln!(
            "[build.rs] Warning: dist/index.html missing after build. Creating fallback dist/index.html"
        );
        let dist_dir = manifest_path.join("dist");
        let _ = fs::create_dir_all(&dist_dir);
        let fallback_html = r#"<!DOCTYPE html><html><head><title>VuIO Media Info</title></head><body><div id="app">Loading...</div></body></html>"#;
        let _ = fs::write(&dist_index, fallback_html);
    }

    // Run tauri build to embed dist/ and generate context
    tauri_build::build();
}
