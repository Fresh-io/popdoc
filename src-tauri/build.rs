use std::path::PathBuf;

fn main() {
    // Load .env from the repo root (one level above src-tauri/).
    let env_path: PathBuf = PathBuf::from("..").join(".env");
    println!("cargo:rerun-if-changed=../.env");
    let _ = dotenvy::from_path(&env_path);

    // Propagate to the crate via env!() at compile time.
    for key in ["GDL_CLIENT_ID", "GDL_CLIENT_SECRET"] {
        let val = std::env::var(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={val}");
    }

    tauri_build::build();
}
