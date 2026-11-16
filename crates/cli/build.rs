use std::{env, fs, path::PathBuf};

fn main() {
    let dist = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("../../ui/dist");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    // Fresh checkouts can build the CLI and MCP without Node. The UI command
    // requires a frontend build followed by Cargo, which embeds these assets.
    for asset in ["index.html", "app.js", "app.css"] {
        let path = dist.join(asset);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(path).unwrap_or_default();
        fs::write(out.join(asset), bytes).expect("write embedded UI asset");
    }
}
