fn main() {
    // Load the repo-root .env into the build as compile-time env vars, so no
    // secret ever lives in source. ponytail: no dotenvy dep — the subset we
    // need is KEY=VALUE, one per line, with optional quotes and # comments.
    println!("cargo:rerun-if-changed=../.env");
    if let Ok(contents) = std::fs::read_to_string("../.env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                println!("cargo:rustc-env={}={}", key.trim(), value);
            }
        }
    }

    tauri_build::build()
}
