//! Exercise the public preflight command without Android/Xcode installations.
use std::{fs, io::Write, process::Command};

#[test]
fn artifact_doctor_checks_the_packaged_core_using_the_project_abi_contract() {
    let root = std::env::temp_dir().join(format!("jffi-artifact-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("core")).unwrap();
    fs::write(
        root.join("jffi.toml"),
        r#"
[package]
name = "sample"
version = "1.0.0"
[platforms]
enabled = ["android"]
[platforms.android]
package = "com.example.sample"
abis = ["x86_64"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("core/Cargo.toml"),
        "[package]\nname = \"custom-core\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let mut elf = vec![0u8; 120];
    elf[..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2;
    elf[5] = 1;
    elf[16] = 3;
    elf[18] = 62;
    elf[32] = 64;
    elf[54] = 56;
    elf[56] = 1;
    elf[64] = 1;
    for (alignment, success) in [(16384u64, true), (4096, false)] {
        elf[112..120].copy_from_slice(&alignment.to_le_bytes());
        let file = fs::File::create(root.join("sample.aab")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("BundleConfig.pb", options).unwrap();
        zip.write_all(&[0x12, 6, 0x12, 4, 8, 1, 16, 2]).unwrap();
        zip.start_file("base/lib/x86_64/libcustom_core.so", options)
            .unwrap();
        zip.write_all(&elf).unwrap();
        zip.finish().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
            .current_dir(&root)
            .args([
                "--no-setup",
                "doctor",
                "artifact",
                "--platform",
                "android",
                "--artifact",
                "sample.aab",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if !success {
            assert!(String::from_utf8_lossy(&output.stderr).contains("LOAD segment"));
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_doctor_rejects_invalid_windows_architecture_selections() {
    let root = std::env::temp_dir().join(format!("jffi-windows-targets-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    for (targets, success) in [
        (
            "[\"aarch64-pc-windows-msvc\",\"x86_64-pc-windows-msvc\"]",
            true,
        ),
        ("[]", false),
        ("[\"aarch64-pc-windows-msvc\",\"typo\"]", false),
        (
            "[\"aarch64-pc-windows-msvc\",\"aarch64-pc-windows-msvc\"]",
            false,
        ),
    ] {
        fs::write(root.join("jffi.toml"),format!("[package]\nname = \"sample\"\nversion = \"1.0.0\"\n[platforms]\nenabled = [\"windows\"]\n[bundle.windows]\ntargets = {targets}\n")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
            .current_dir(&root)
            .args(["--no-setup", "doctor", "config"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::remove_dir_all(root).unwrap();
}
