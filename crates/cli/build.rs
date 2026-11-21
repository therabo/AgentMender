use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let dist = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("../../ui/dist");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    // Embed the built frontend. The UI command reports missing assets at launch.
    for asset in ["index.html", "app.js", "app.css"] {
        let path = dist.join(asset);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(path).unwrap_or_default();
        fs::write(out.join(asset), bytes).expect("write embedded UI asset");
    }

    let assets = dist.join("assets");
    println!("cargo:rerun-if-changed={}", assets.display());
    let mut entries = match fs::read_dir(assets) {
        Ok(entries) => entries
            .map(|entry| entry.expect("read UI asset directory").path())
            .collect::<Vec<_>>(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("read UI assets: {error}"),
    };
    entries.sort_unstable();
    let mut lookup = String::from(
        "fn embedded_asset(name: &str) -> Option<(&'static str, &'static [u8])> {\n    match name {\n",
    );
    for path in entries {
        let content_type = match path.extension().and_then(|extension| extension.to_str()) {
            Some("svg") => "image/svg+xml",
            Some("woff2") => "font/woff2",
            Some("js") => "text/javascript; charset=utf-8",
            _ => continue,
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UI asset name");
        writeln!(
            lookup,
            "        {name:?} => Some(({content_type:?}, include_bytes!({path:?}))),"
        )
        .expect("generate embedded asset lookup");
    }
    lookup.push_str("        _ => None,\n    }\n}\n");
    fs::write(out.join("embedded_assets.rs"), lookup).expect("write embedded asset lookup");
}
