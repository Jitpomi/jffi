//! Decode the metadata format matching the UniFFI version resolved for Core.
use super::*;
mod v029 {
    use uniffi_meta as version;
    const MODERN: bool = false;
    include!("schema.rs");
}
mod v031 {
    use uniffi_meta_031 as version;
    const MODERN: bool = true;
    include!("schema.rs");
}
pub(super) fn load(root: &Path, release: bool, library: &str) -> Result<Api> {
    let output = std::process::Command::new("cargo")
        .current_dir(root)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            "wasm32-unknown-unknown",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Resolving UniFFI metadata version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let versions: Vec<_> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["name"] == "uniffi")
        .filter_map(|p| p["version"].as_str())
        .collect();
    let mut api = match versions.as_slice() {
        [version] if version.starts_with("0.29.") => v029::load(root, release, library),
        [version] if version.starts_with("0.31.") => v031::load(root, release, library),
        _ => bail!(
            "Web metadata currently supports a single UniFFI 0.29.x or 0.31.x version; resolved {}",
            versions.join(", ")
        ),
    }?;
    // Cargo resolves aliases and workspace/target inheritance. Only the public
    // scaffolding tag anchors linking; no user-defined Rust paths are imported.
    let packages = metadata["packages"]
        .as_array()
        .context("Cargo packages missing")?;
    let adapter = root.join("ffi-web/Cargo.toml").canonicalize()?;
    let core = root.join("core/Cargo.toml").canonicalize()?;
    let package_id = |path: &Path| {
        packages
            .iter()
            .find(|p| {
                p["manifest_path"]
                    .as_str()
                    .is_some_and(|p| Path::new(p) == path)
            })
            .map(|p| p["id"].clone())
            .context("Web/Core Cargo package missing")
    };
    let adapter_id = package_id(&adapter)?;
    let core_id = package_id(&core)?;
    let alias = metadata["resolve"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["id"] == adapter_id))
        .and_then(|n| n["deps"].as_array())
        .and_then(|deps| deps.iter().find(|d| d["pkg"] == core_id))
        .and_then(|d| d["name"].as_str())
        .context("Web adapter Core dependency missing")?;
    api.module_alias = Some(format_ident!("{alias}"));
    Ok(api)
}
