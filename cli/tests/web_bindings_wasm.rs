//! Opt-in real WASM regression; the default tests cover generation without tool installs.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn succeeds(output: Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore = "requires wasm32 target, Node, Python, wasm-pack, matching UniFFI bindgen and wasm-bindgen-cli 0.2.129"]
fn exported_types_round_trip_through_actual_wasm() {
    let root = std::env::temp_dir().join(format!("jffi-web-wasm-{}", uuid::Uuid::new_v4()));
    let project = root.join("simple");
    succeeds(
        Command::new(env!("CARGO_BIN_EXE_jffi"))
            .args([
                "new",
                "simple",
                "--platforms",
                "web",
                "--template",
                "hello",
                "--no-build",
                "--path",
            ])
            .arg(&project)
            .output()
            .unwrap(),
    );
    fs::write(project.join("core/Cargo.toml"), "[package]\nname='simple-core'\nversion='0.1.0'\nedition='2021'\n[features]\ncompat-renames=[]\nweb-extra=[]\n[lib]\ncrate-type=['cdylib','staticlib','lib']\n[dependencies]\nasync-trait='0.1'\nuniffi={version='=0.29.4',features=['wasm-unstable-single-threaded']}\n").unwrap();
    fs::create_dir_all(project.join("shared/src")).unwrap();
    fs::write(project.join("shared/Cargo.toml"), "[package]\nname='shared-api'\nversion='0.1.0'\nedition='2021'\n[features]\nresolved-feature=[]\n[dependencies]\nuniffi={version='=0.29.4',features=['wasm-unstable-single-threaded']}\n").unwrap();
    fs::write(project.join("shared/src/lib.rs"), "#[derive(Clone,Debug)] pub struct SharedId(pub u64); uniffi::custom_newtype!(SharedId,u64); #[derive(uniffi::Record)] pub struct SharedRecord { pub id: SharedId, #[cfg(feature=\"resolved-feature\")] pub label:String } uniffi::setup_scaffolding!();").unwrap();
    let shared_source = project.join("shared/src/lib.rs");
    let mut source = fs::read_to_string(&shared_source).unwrap();
    source.push_str(
        r#"
#[macro_export] macro_rules! legacy_export {
    ($name:ident,$function:ident,$($field:ident),+) => {
        mod arbitrary_macro_output {
            #[derive(uniffi::Record)] struct $name { $($field:String),+ }
            #[uniffi::export] fn $function(value:$name)->$name {value}
        }
    };
}
"#,
    );
    fs::write(shared_source, source).unwrap();
    // Core sees a facade; only the facade directly depends on the exported crate.
    fs::create_dir_all(project.join("facade/src")).unwrap();
    fs::write(project.join("facade/Cargo.toml"), "[package]\nname='compat-facade'\nversion='0.1.0'\nedition='2021'\n[workspace]\n[workspace.dependencies]\nshared-alias={package='shared-api',path='../shared'}\n[dependencies]\nshared-alias={workspace=true,features=['resolved-feature']}\n").unwrap();
    fs::write(
        project.join("facade/src/lib.rs"),
        "pub use shared_alias::*;",
    )
    .unwrap();
    let core_manifest = project.join("core/Cargo.toml");
    let mut source = fs::read_to_string(&core_manifest).unwrap();
    source.push_str("\n[target.'cfg(target_arch=\"wasm32\")'.dependencies]\nshared-alias={package='compat-facade',path='../facade'}\n[target.'cfg(not(target_arch=\"wasm32\"))'.dependencies]\nshared-alias={package='compat-facade',path='../facade'}\n");
    let manifest = project.join("Cargo.toml");
    let mut workspace: toml_edit::DocumentMut =
        fs::read_to_string(&manifest).unwrap().parse().unwrap();
    let mut exclusions = toml_edit::Array::new();
    exclusions.push("facade");
    workspace["workspace"]["exclude"] = toml_edit::value(exclusions);
    let mut dependency = toml_edit::InlineTable::new();
    dependency.insert("package", "shared-api".into());
    dependency.insert("path", "shared".into());
    workspace["workspace"]["dependencies"]["shared-alias"] = toml_edit::value(dependency);
    fs::write(manifest, workspace.to_string()).unwrap();
    fs::write(core_manifest, source).unwrap();
    let uniffi_version =
        std::env::var("JFFI_COMPAT_UNIFFI_VERSION").unwrap_or_else(|_| "0.29.4".into());
    for manifest in ["core/Cargo.toml", "shared/Cargo.toml"] {
        let path = project.join(manifest);
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("=0.29.4", &format!("={uniffi_version}"));
        fs::write(path, text).unwrap();
    }
    if uniffi_version.starts_with("0.31.") {
        let path = project.join("core/Cargo.toml");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("[features]", "[features]\ndefault=['compat-renames']");
        fs::write(path, text).unwrap();
    }
    fs::write(project.join("core/build.rs"), r##"fn main(){let out=std::env::var("OUT_DIR").unwrap();std::fs::write(std::path::Path::new(&out).join("web_generated.rs"),"#[cfg(web_build_generated)] #[uniffi::export] pub fn build_generated_value()->i32{91}").unwrap();println!("cargo:rustc-cfg=web_build_generated");println!("cargo:rustc-check-cfg=cfg(web_build_generated)");}"##).unwrap();
    let core = r#"
#[derive(uniffi::Object)]
pub struct Core { count: std::sync::Mutex<i32> }
#[uniffi::export]
impl Core {
    #[uniffi::constructor] pub fn new() -> Self { Self { count: std::sync::Mutex::new(0) } }
    pub fn greeting(&self) -> String { "Hello from JFFI".into() }
    pub fn get_count(&self) -> i32 { *self.count.lock().unwrap() }
    pub fn increment(&self) -> i32 { let mut value=self.count.lock().unwrap(); *value+=1; *value }
}
uniffi::setup_scaffolding!();
pub mod web_types;
pub use crate::web_types::{WebChoice, WebFailure, WebProfile, WebCounter, WebObjectEnvelope};
"#;
    fs::write(project.join("core/src/lib.rs"), core).unwrap();
    fs::write(
        project.join("core/src/web_types.rs"),
        format!(
            "{}\n{}\n{}\n{}\n{}",
            include_str!("fixtures/web_types.rs"),
            include_str!("fixtures/web_callbacks.rs"),
            include_str!("fixtures/web_functions.rs"),
            include_str!("fixtures/web_compat.rs"),
            include_str!("fixtures/web_private.rs")
        ),
    )
    .unwrap();
    fs::write(
        project.join("core/src/web_included.rs"),
        include_str!("fixtures/web_included.rs"),
    )
    .unwrap();
    let manifest = project.join("ffi-web/Cargo.toml");
    let mut doc: toml_edit::DocumentMut = fs::read_to_string(&manifest).unwrap().parse().unwrap();
    doc["dependencies"]["wasm-bindgen"] = toml_edit::value("=0.2.129");
    let mut core_dependency = doc["dependencies"]
        .as_table_like_mut()
        .unwrap()
        .remove("simple-core")
        .unwrap();
    core_dependency["package"] = toml_edit::value("simple-core");
    core_dependency["path"] = toml_edit::value("core");
    let workspace_path = project.join("Cargo.toml");
    let mut workspace: toml_edit::DocumentMut = fs::read_to_string(&workspace_path)
        .unwrap()
        .parse()
        .unwrap();
    workspace["workspace"]["dependencies"]["core-alias"] = core_dependency;
    fs::write(workspace_path, workspace.to_string()).unwrap();
    let mut inherited = toml_edit::InlineTable::new();
    inherited.insert("workspace", true.into());
    doc["dependencies"]["core-alias"] = toml_edit::value(inherited);
    fs::write(manifest, doc.to_string()).unwrap();
    succeeds(
        Command::new("cargo")
            .arg("generate-lockfile")
            .current_dir(&project)
            .output()
            .unwrap(),
    );
    #[cfg(unix)]
    if let Some(target) = std::env::var_os("JFFI_WASM_TEST_TARGET_DIR") {
        std::os::unix::fs::symlink(target, project.join("target")).unwrap();
    }
    let build = || {
        let rust_before = fs::read(project.join("core/src/lib.rs")).unwrap();
        let fixture_before = fs::read(project.join("core/src/web_types.rs")).ok();
        let mut command = Command::new(env!("CARGO_BIN_EXE_jffi"));
        command
            .args(["build", "--platform", "web", "--no-setup"])
            .current_dir(&project);
        succeeds(command.output().unwrap());
        assert_eq!(
            rust_before,
            fs::read(project.join("core/src/lib.rs")).unwrap(),
            "web builds must not rewrite Core source"
        );
        assert_eq!(
            fixture_before,
            fs::read(project.join("core/src/web_types.rs")).ok(),
            "web builds must not rewrite private/macro source"
        );
    };
    build();
    succeeds(
        Command::new("node")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_functions.mjs"))
            .arg(project.join("platforms/web/pkg"))
            .output()
            .unwrap(),
    );
    succeeds(
        Command::new("node")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_private.mjs"))
            .arg(project.join("platforms/web/pkg"))
            .output()
            .unwrap(),
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_types.mjs");
    succeeds(
        Command::new("node")
            .arg(script)
            .arg(project.join("platforms/web/pkg"))
            .output()
            .unwrap(),
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_callbacks.mjs");
    succeeds(
        Command::new("node")
            .arg(script)
            .arg(project.join("platforms/web/pkg"))
            .output()
            .unwrap(),
    );
    succeeds(
        Command::new("cargo")
            .args(["test", "--workspace"])
            .current_dir(&project)
            .output()
            .unwrap(),
    );
    if let Some(tsc) = std::env::var_os("JFFI_TYPESCRIPT_TSC") {
        for fixture in [
            "web_types.ts",
            "web_callbacks.ts",
            "web_compat.ts",
            "web_private.ts",
        ] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(fixture),
                project.join("platforms/web").join(fixture),
            )
            .unwrap();
        }
        succeeds(
            Command::new("node")
                .arg(tsc)
                .args([
                    "--strict",
                    "--noEmit",
                    "--target",
                    "ES2020",
                    "--lib",
                    "ES2020,DOM,ESNext.Disposable",
                    "--moduleResolution",
                    "bundler",
                    "--module",
                    "ESNext",
                    "web_types.ts",
                    "web_callbacks.ts",
                    "web_compat.ts",
                    "web_private.ts",
                ])
                .current_dir(project.join("platforms/web"))
                .output()
                .unwrap(),
        );
    }
    let wasm_comparison = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_compat.mjs"))
        .arg(project.join("platforms/web/pkg"))
        .output()
        .unwrap();
    assert!(
        wasm_comparison.status.success(),
        "{}",
        String::from_utf8_lossy(&wasm_comparison.stderr)
    );
    succeeds(
        Command::new("cargo")
            .args(["build", "--package", "simple-core"])
            .current_dir(&project)
            .output()
            .unwrap(),
    );
    let native = project.join("native_compatibility");
    fs::create_dir_all(&native).unwrap();
    fs::write(native.join("__init__.py"), "").unwrap();
    let library = if cfg!(target_os = "macos") {
        project.join("target/debug/libsimple_core.dylib")
    } else if cfg!(target_os = "windows") {
        project.join("target/debug/simple_core.dll")
    } else {
        project.join("target/debug/libsimple_core.so")
    };
    succeeds(
        Command::new("uniffi-bindgen")
            .args(["generate", "--library"])
            .arg(&library)
            .args(["--language", "python", "--out-dir"])
            .arg(&native)
            .output()
            .unwrap(),
    );
    fs::copy(&library, native.join(library.file_name().unwrap())).unwrap();
    let native_comparison = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_compat.py"))
        .env("PYTHONPATH", &project)
        .output()
        .unwrap();
    assert!(
        native_comparison.status.success(),
        "{}",
        String::from_utf8_lossy(&native_comparison.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&wasm_comparison.stdout).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&native_comparison.stdout).unwrap(),
        "WASM and generated native UniFFI bindings differ"
    );
    let declarations =
        fs::read_to_string(project.join("platforms/web/pkg/simple_ffi_web.d.ts")).unwrap();
    for expected in [
        "export type WebEmail = string",
        "export type WebUserId = bigint",
        "export interface WebFormatter",
        "export interface WebService",
        "transformAsync(value: WebCustomRecord): Promise<WebCustomRecord>",
        "retainFormatter(callback: WebFormatter): void",
        "export type WebProfile",
        "bigCount: bigint",
        "Map<string, boolean>",
        "asyncProfile(value: WebProfile): Promise<WebProfile>",
        "export class WebCounter",
        "roundTripObject(counter: WebCounter): WebCounter",
    ] {
        assert!(
            declarations.contains(expected),
            "Missing {expected}\n{declarations}"
        );
    }
    let adapter_manifest = project.join("ffi-web/Cargo.toml");
    let adapter_original = fs::read_to_string(&adapter_manifest).unwrap();
    let mut adapter: toml_edit::DocumentMut = adapter_original.parse().unwrap();
    let mut features = toml_edit::Array::new();
    features.push("web-extra");
    adapter["dependencies"]["core-alias"]["features"] = toml_edit::value(features);
    fs::write(&adapter_manifest, adapter.to_string()).unwrap();
    build();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_functions.mjs");
    succeeds(
        Command::new("node")
            .arg(&script)
            .arg(project.join("platforms/web/pkg"))
            .arg("--feature")
            .output()
            .unwrap(),
    );
    succeeds(
        Command::new(env!("CARGO_BIN_EXE_jffi"))
            .args(["build", "--platform", "web", "--no-setup", "--release"])
            .current_dir(&project)
            .output()
            .unwrap(),
    );
    succeeds(
        Command::new("node")
            .arg(&script)
            .arg(project.join("platforms/web/pkg"))
            .args(["--feature", "--release"])
            .output()
            .unwrap(),
    );
    fs::write(adapter_manifest, adapter_original).unwrap();
    // UniFFI 0.31.1 itself fails to compile wasm32 futures without its
    // wasm-unstable-single-threaded feature (E0277 in UniffiCompatibleFuture).
    // Keep that version in its supported local-future mode; test both modes on 0.29.
    if uniffi_version.starts_with("0.29.") {
        // Also verify native-style Send callback futures, without UniFFI's local-future feature.
        let core_manifest = project.join("core/Cargo.toml");
        let source = fs::read_to_string(&core_manifest)
            .unwrap()
            .replace(",features=['wasm-unstable-single-threaded']", "");
        fs::write(core_manifest, source).unwrap();
        let shared_manifest = project.join("shared/Cargo.toml");
        let source = fs::read_to_string(&shared_manifest)
            .unwrap()
            .replace(",features=['wasm-unstable-single-threaded']", "");
        fs::write(shared_manifest, source).unwrap();
        let fixture = project.join("core/src/web_types.rs");
        let source = fs::read_to_string(&fixture).unwrap().replace(
            "async_trait::async_trait(?Send)",
            "async_trait::async_trait",
        );
        fs::write(fixture, source).unwrap();
        build();
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web_callbacks.mjs");
        succeeds(
            Command::new("node")
                .arg(script)
                .arg(project.join("platforms/web/pkg"))
                .output()
                .unwrap(),
        );
    }
    // Removing the schema from Core must remove it from both JS and .d.ts.
    fs::write(
        project.join("core/src/lib.rs"),
        core.split("pub mod web_types;").next().unwrap(),
    )
    .unwrap();
    build();
    let declarations =
        fs::read_to_string(project.join("platforms/web/pkg/simple_ffi_web.d.ts")).unwrap();
    assert!(!declarations.contains("WebProfile"));
    assert!(!declarations.contains("WebCounter"));
    assert!(!declarations.contains("WebFormatter"));
    assert!(!declarations.contains("WebEmail"));
    fs::write(project.join("core/src/lib.rs"), "#[uniffi::export] pub fn standalone_add(left: i32, right: i32) -> i32 {left + right} uniffi::setup_scaffolding!();").unwrap();
    build();
    let declarations =
        fs::read_to_string(project.join("platforms/web/pkg/simple_ffi_web.d.ts")).unwrap();
    assert!(declarations.contains("export function standaloneAdd"));
    assert!(!declarations.contains("export class Core"));
    fs::remove_dir_all(root).unwrap();
}
