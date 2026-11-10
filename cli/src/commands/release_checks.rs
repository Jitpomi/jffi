//! Checks of built artifacts, rather than assumptions about compiler flags.
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeSet, fs::File, io::Read, path::Path, process::Command};

pub(crate) fn android_native_package(path: &Path, abis: &[String], core: &str) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(path)?)
        .with_context(|| format!("Invalid Android package: {}", path.display()))?;
    let prefix = if path.extension().is_some_and(|e| e == "aab") {
        "base/lib/"
    } else {
        "lib/"
    };
    if prefix == "base/lib/" {
        const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
        let mut config = Vec::new();
        zip.by_name("BundleConfig.pb")
            .context("AAB is missing BundleConfig.pb")?
            .take(MAX_CONFIG_BYTES + 1)
            .read_to_end(&mut config)?;
        ensure!(config.len() as u64 <= MAX_CONFIG_BYTES,
            "BundleConfig.pb exceeds the 1 MiB inspection limit; refusing to validate a truncated configuration");
        aab_alignment(&config)?;
    }
    let mut cores = BTreeSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        // Include libraries from feature modules in AABs, not only the base module.
        let name = entry.name()?.into_owned();
        let native = if prefix == "base/lib/" {
            name.split_once('/').and_then(|(module, contents)| {
                if matches!(module, "BUNDLE-METADATA" | "META-INF") {
                    None
                } else {
                    contents.strip_prefix("lib/")
                }
            })
        } else {
            name.strip_prefix(prefix)
        };
        let Some((abi, file)) = native.and_then(|p| p.split_once('/')) else {
            continue;
        };
        if !file.ends_with(".so") {
            continue;
        }
        ensure!(
            !file.contains('/'),
            "Unexpected native library path: {name}"
        );
        ensure!(
            abis.iter().any(|a| a == abi),
            "Unconfigured ABI {abi} in {} ({name})",
            path.display()
        );
        let mut header = Vec::new();
        // ELF program headers are small. Do not load multi-gigabyte libraries into memory.
        entry.by_ref().take(65536).read_to_end(&mut header)?;
        elf_library(&header, abi).with_context(|| format!("{}: {name}", path.display()))?;
        if name == format!("{prefix}{abi}/{core}") {
            cores.insert(abi.to_owned());
        }
    }
    for abi in abis {
        ensure!(
            cores.contains(abi),
            "{} is missing {prefix}{abi}/{core}",
            path.display()
        );
    }
    Ok(())
}

// Read just the public BundleConfig protobuf fields needed for native alignment.
// Schema: google/bundletool src/main/proto/config.proto (Optimizations=2,
// UncompressNativeLibraries=2, enabled=1, alignment=2). Unknown fields are skipped.
fn protobuf_field(mut data: &[u8], field: u64, expected_wire: u64) -> Result<Option<&[u8]>> {
    fn varint(data: &mut &[u8]) -> Result<u64> {
        let mut value = 0;
        for shift in (0..70).step_by(7) {
            let (&byte, rest) = data.split_first().context("Truncated protobuf varint")?;
            *data = rest;
            ensure!(shift < 63 || byte <= 1, "Protobuf varint overflow");
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                return Ok(value);
            }
        }
        anyhow::bail!("Invalid protobuf varint")
    }
    let mut found = None;
    while !data.is_empty() {
        let key = varint(&mut data)?;
        ensure!(key >> 3 != 0, "Invalid protobuf field number");
        let wire = key & 7;
        let value = match wire {
            0 => {
                let before = data;
                varint(&mut data)?;
                &before[..before.len() - data.len()]
            }
            1 | 2 | 5 => {
                let length = match wire {
                    1 => 8,
                    5 => 4,
                    _ => usize::try_from(varint(&mut data)?).context("Protobuf length overflow")?,
                };
                let value = data.get(..length).context("Truncated protobuf field")?;
                data = &data[length..];
                value
            }
            _ => anyhow::bail!("Unsupported protobuf wire type {wire}"),
        };
        if key >> 3 == field {
            ensure!(
                wire == expected_wire && found.is_none(),
                "Invalid or duplicate BundleConfig field {field}"
            );
            found = Some(value);
        }
    }
    Ok(found)
}

fn aab_alignment(data: &[u8]) -> Result<()> {
    let Some(optimizations) = protobuf_field(data, 2, 2)? else {
        return Ok(());
    };
    let Some(native) = protobuf_field(optimizations, 2, 2)? else {
        return Ok(());
    };
    // Compressed libraries are extracted rather than mmap'ed from the ZIP.
    // Their ELF alignment still needs checking, which is done for every library.
    if protobuf_field(native, 1, 0)?.is_some_and(|v| v != [0]) {
        ensure!(protobuf_field(native, 2, 0)?.is_some_and(|v| v == [2] || v == [3]),
            "AAB requests uncompressed native libraries without 16 KB/64 KB ZIP alignment; update Android Gradle Plugin to 8.5.1 or later");
    }
    Ok(())
}

fn number(bytes: &[u8], offset: usize, width: usize) -> Result<u64> {
    let field = bytes
        .get(offset..offset.checked_add(width).context("ELF offset overflow")?)
        .context("Truncated ELF header")?;
    let mut value = 0u64;
    for (shift, byte) in field.iter().enumerate() {
        value |= u64::from(*byte) << (shift * 8);
    }
    Ok(value)
}

fn elf_library(bytes: &[u8], abi: &str) -> Result<()> {
    ensure!(
        bytes.get(..4) == Some(b"\x7fELF"),
        "Native library is not ELF"
    );
    let (class, machine) = match abi {
        "arm64-v8a" => (2, 183),
        "armeabi-v7a" => (1, 40),
        "x86_64" => (2, 62),
        _ => anyhow::bail!("Unsupported Android ABI: {abi}"),
    };
    ensure!(
        bytes.get(4) == Some(&class) && bytes.get(5) == Some(&1),
        "Wrong ELF class or byte order for {abi}"
    );
    ensure!(
        number(bytes, 18, 2)? == machine && number(bytes, 16, 2)? == 3,
        "Wrong ELF architecture/type for {abi}"
    );
    let (phoff, size, count) = if class == 2 {
        (
            number(bytes, 32, 8)?,
            number(bytes, 54, 2)?,
            number(bytes, 56, 2)?,
        )
    } else {
        (
            number(bytes, 28, 4)?,
            number(bytes, 42, 2)?,
            number(bytes, 44, 2)?,
        )
    };
    ensure!(
        size >= if class == 2 { 56 } else { 32 },
        "Invalid ELF program header size"
    );
    let end = phoff
        .checked_add(size.checked_mul(count).context("ELF header overflow")?)
        .context("ELF header overflow")?;
    ensure!(
        end <= bytes.len() as u64,
        "Truncated or oversized ELF program headers"
    );
    let mut loads = 0;
    for i in 0..count {
        let start = (phoff + i * size) as usize;
        if number(bytes, start, 4)? != 1 {
            continue;
        }
        loads += 1;
        if class == 2 {
            let alignment = number(bytes, start + 48, 8)?;
            let offset = number(bytes, start + 8, 8)?;
            let address = number(bytes, start + 16, 8)?;
            ensure!(alignment >= 16384 && alignment.is_power_of_two() && offset % 16384 == address % 16384,
                "{abi} LOAD segment does not support 16 KB pages; rebuild this library/SDK with a current NDK");
        }
    }
    ensure!(loads > 0, "ELF library has no LOAD segments");
    Ok(())
}

/// Only App Store exports have a submission SDK floor. Development/website
/// exports remain usable with older Xcode. Policy checked November 2026.
pub(crate) fn apple_store_toolchain(platform: &str) -> Result<()> {
    let xcode = output("xcodebuild", &["-version"])?;
    let sdk = output(
        "xcrun",
        &[
            "--sdk",
            if platform == "ios" {
                "iphoneos"
            } else {
                "macosx"
            },
            "--show-sdk-version",
        ],
    )?;
    let developer = match std::env::var("DEVELOPER_DIR") {
        Ok(path) => path,
        Err(_) => output("xcode-select", &["-p"])?,
    };
    apple_sdk_policy(platform, &xcode, &sdk, &developer)
}

fn output(tool: &str, args: &[&str]) -> Result<String> {
    let result = Command::new(tool)
        .args(args)
        .output()
        .with_context(|| format!("Could not run {tool}"))?;
    ensure!(
        result.status.success(),
        "{tool} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}

fn apple_sdk_policy(platform: &str, xcode: &str, sdk: &str, developer: &str) -> Result<()> {
    let selected = format!("{xcode} {developer}").to_ascii_lowercase();
    ensure!(
        !selected.contains("beta")
            && !selected.contains("candidate")
            && !selected.contains("_rc")
            && !selected.contains("-rc"),
        "App Store bundles require a stable Xcode release"
    );
    let major = |v: &str| v.split('.').next().and_then(|v| v.parse::<u32>().ok());
    let xcode_version = xcode
        .strip_prefix("Xcode ")
        .and_then(|v| v.split_whitespace().next())
        .and_then(major);
    ensure!(
        xcode_version.is_some(),
        "Could not identify the selected Xcode version"
    );
    // Apple's current iOS/iPadOS floor is 26; 27 becomes mandatory in April 2027.
    if platform == "ios" {
        ensure!(
            xcode_version.is_some_and(|v| v >= 26),
            "App Store iOS bundles require Xcode 26 or later"
        );
        ensure!(
            major(sdk).is_some_and(|v| v >= 26),
            "App Store iOS bundles require the iOS 26 SDK or later (27 from April 2027)"
        );
    }
    Ok(())
}

/// Inspect the SDK embedded by Xcode as well: a project may override SDKROOT.
pub(crate) fn apple_store_archive(app: &Path, platform: &str) -> Result<()> {
    let plist = app.join(if platform == "ios" {
        "Info.plist"
    } else {
        "Contents/Info.plist"
    });
    let data = output(
        "plutil",
        &[
            "-convert",
            "json",
            "-o",
            "-",
            plist.to_str().context("Non-UTF8 Apple archive path")?,
        ],
    )?;
    let info: serde_json::Value = serde_json::from_str(&data)?;
    if platform == "ios" {
        let sdk = info["DTSDKName"]
            .as_str()
            .and_then(|s| s.strip_prefix("iphoneos"))
            .context("App Store archive must use the device iOS SDK, not a simulator SDK")?;
        ensure!(
            sdk.split('.')
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|v| v >= 26),
            "Archived app requires iOS SDK 26 or later"
        );
        if app.extension().is_some_and(|e| e == "app")
            && sdk
                .split('.')
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|v| v >= 27)
        {
            ensure!(
                info.get("UILaunchScreen").is_some()
                    || info.get("UILaunchStoryboardName").is_some()
                    || info.get("UILaunchStoryboards").is_some()
                    || info.get("UILaunchScreens").is_some(),
                "iOS 27 archives require a launch screen in Info.plist"
            );
        }
        let plugins = app.join("PlugIns");
        if plugins.is_dir() {
            for extension in std::fs::read_dir(plugins)? {
                let path = extension?.path();
                if path.extension().is_some_and(|e| e == "appex") {
                    apple_store_archive(&path, platform)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn elf(abi: &str, alignment: u64) -> Vec<u8> {
        let (class, machine) = if abi == "arm64-v8a" {
            (2, 183u16)
        } else if abi == "x86_64" {
            (2, 62)
        } else {
            (1, 40)
        };
        let mut b = vec![0u8; 120];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = class;
        b[5] = 1;
        b[16..18].copy_from_slice(&3u16.to_le_bytes());
        b[18..20].copy_from_slice(&machine.to_le_bytes());
        if class == 2 {
            b[32..40].copy_from_slice(&64u64.to_le_bytes());
            b[54] = 56;
            b[56] = 1;
            b[64] = 1;
            b[112..120].copy_from_slice(&alignment.to_le_bytes());
        } else {
            b[28] = 64;
            b[42] = 32;
            b[44] = 1;
            b[64] = 1;
        }
        b
    }
    #[test]
    fn validates_every_64_bit_load_segment_and_architecture() {
        for abi in ["arm64-v8a", "x86_64"] {
            assert!(elf_library(&elf(abi, 16384), abi).is_ok());
            assert!(elf_library(&elf(abi, 4096), abi).is_err());
            assert!(elf_library(&elf(abi, 24576), abi).is_err());
            let mut b = elf(abi, 16384);
            b[72] = 1;
            assert!(elf_library(&b, abi).is_err());
        }
        assert!(elf_library(&elf("armeabi-v7a", 4096), "armeabi-v7a").is_ok());
        assert!(elf_library(&elf("x86_64", 16384), "arm64-v8a").is_err());
        assert!(elf_library(b"broken", "arm64-v8a").is_err());
        let mut b = elf("arm64-v8a", 16384);
        b[32..40].fill(255);
        assert!(elf_library(&b, "arm64-v8a").is_err());
    }
    fn package(entries: &[(&str, Vec<u8>)], ext: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("jffi-native-{}.{}", uuid::Uuid::new_v4(), ext));
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        if ext == "aab" && !entries.iter().any(|(name, _)| *name == "BundleConfig.pb") {
            zip.start_file("BundleConfig.pb", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&[0x12, 6, 0x12, 4, 8, 1, 16, 2]).unwrap();
        }
        for (name, body) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
        path
    }
    #[test]
    fn packages_reject_missing_core_extra_abis_and_unaligned_dependencies() {
        for (ext, prefix) in [("apk", "lib"), ("aab", "base/lib")] {
            let core = format!("{prefix}/arm64-v8a/libsample.so");
            let abis = vec!["arm64-v8a".into()];
            let path = package(&[(&core, elf("arm64-v8a", 16384))], ext);
            assert!(android_native_package(&path, &abis, "libsample.so").is_ok());
            assert!(android_native_package(&path, &abis, "libmissing.so").is_err());
            assert!(android_native_package(&path, &["x86_64".into()], "libsample.so").is_err());
            std::fs::remove_file(path).unwrap();
            let dependency = if ext == "aab" {
                "feature/lib/arm64-v8a/libjna.so"
            } else {
                "lib/arm64-v8a/libjna.so"
            };
            let path = package(
                &[
                    (&core, elf("arm64-v8a", 16384)),
                    (dependency, elf("arm64-v8a", 4096)),
                ],
                ext,
            );
            assert!(android_native_package(&path, &abis, "libsample.so").is_err());
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn aab_assets_and_metadata_are_not_native_module_libraries() {
        let path = package(
            &[
                ("base/lib/arm64-v8a/libsample.so", elf("arm64-v8a", 16384)),
                (
                    "base/assets/lib/arm64-v8a/fixture.so",
                    b"opaque fixture".to_vec(),
                ),
                (
                    "feature/assets/lib/x86_64/fixture.so",
                    b"opaque fixture".to_vec(),
                ),
                (
                    "BUNDLE-METADATA/lib/x86_64/fixture.so",
                    b"opaque metadata".to_vec(),
                ),
                ("feature/lib/arm64-v8a/libextra.so", elf("arm64-v8a", 16384)),
            ],
            "aab",
        );
        assert!(android_native_package(&path, &["arm64-v8a".into()], "libsample.so").is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn aab_config_limit_rejects_truncation_before_reading_alignment() {
        fn varint(mut value: usize) -> Vec<u8> {
            let mut result = Vec::new();
            while value >= 128 {
                result.push((value as u8 & 127) | 128);
                value >>= 7;
            }
            result.push(value as u8);
            result
        }
        fn field(tag: u8, body: &[u8]) -> Vec<u8> {
            let mut result = vec![tag];
            result.extend(varint(body.len()));
            result.extend(body);
            result
        }
        // A valid Compression.uncompressed_glob field fills exactly the limit.
        let padding = field(26, &field(10, &vec![b'a'; 1024 * 1024 - 8]));
        assert_eq!(padding.len(), 1024 * 1024);
        for oversized in [false, true] {
            let mut config = padding.clone();
            if oversized {
                config.extend([18, 6, 18, 4, 8, 1, 16, 1]);
            }
            let path = package(
                &[
                    ("BundleConfig.pb", config),
                    ("base/lib/arm64-v8a/libsample.so", elf("arm64-v8a", 16384)),
                ],
                "aab",
            );
            let result = android_native_package(&path, &["arm64-v8a".into()], "libsample.so");
            if oversized {
                assert!(result.unwrap_err().to_string().contains("inspection limit"));
            } else {
                assert!(result.is_ok());
            }
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn aab_native_zip_alignment_is_separate_from_elf_alignment() {
        assert!(aab_alignment(&[0x12, 6, 0x12, 4, 8, 1, 16, 2]).is_ok());
        assert!(aab_alignment(&[0x12, 6, 0x12, 4, 8, 1, 16, 3]).is_ok());
        assert!(aab_alignment(&[0x12, 6, 0x12, 4, 8, 1, 16, 1]).is_err());
        assert!(aab_alignment(&[0x12, 4, 0x12, 2, 8, 1]).is_err());
        assert!(aab_alignment(&[0x12, 4, 0x12, 2, 8, 0]).is_ok());
        assert!(aab_alignment(&[0x12, 255]).is_err());
        assert!(aab_alignment(&[0x12, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn apple_archives_check_the_embedded_sdk_and_extension_instead_of_only_the_selected_toolchain()
    {
        let app = std::env::temp_dir().join(format!("jffi-sdk-{}.app", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&app).unwrap();
        let plist = |sdk: &str, launch: bool| {
            format!("<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>DTSDKName</key><string>{sdk}</string>{}</dict></plist>",if launch { "<key>UILaunchScreen</key><dict/>" } else { "" })
        };
        std::fs::write(app.join("Info.plist"), plist("iphoneos27.1", true)).unwrap();
        assert!(apple_store_archive(&app, "ios").is_ok());
        for key in [
            "UILaunchScreens",
            "UILaunchStoryboardName",
            "UILaunchStoryboards",
        ] {
            std::fs::write(
                app.join("Info.plist"),
                plist("iphoneos27.1", true).replace("UILaunchScreen", key),
            )
            .unwrap();
            assert!(apple_store_archive(&app, "ios").is_ok(), "{key}");
        }
        std::fs::write(app.join("Info.plist"), plist("iphoneos27.1", false)).unwrap();
        assert!(apple_store_archive(&app, "ios").is_err());
        std::fs::write(app.join("Info.plist"), plist("iphoneos26.0", false)).unwrap();
        assert!(apple_store_archive(&app, "ios").is_ok());
        let extension = app.join("PlugIns/Share.appex");
        std::fs::create_dir_all(&extension).unwrap();
        std::fs::write(
            extension.join("Info.plist"),
            plist("iphonesimulator26.0", false),
        )
        .unwrap();
        assert!(apple_store_archive(&app, "ios").is_err());
        std::fs::write(extension.join("Info.plist"), plist("iphoneos27.1", false)).unwrap();
        assert!(apple_store_archive(&app, "ios").is_ok());
        std::fs::remove_dir_all(app).unwrap();
    }

    #[test]
    fn apple_submission_floor_does_not_prematurely_require_sdk_27() {
        assert!(apple_sdk_policy(
            "ios",
            "Xcode 26.4\nBuild version 17F",
            "26.4",
            "/Applications/Xcode.app"
        )
        .is_ok());
        assert!(apple_sdk_policy("ios", "Xcode 27.1", "27.1", "/Applications/Xcode.app").is_ok());
        assert!(apple_sdk_policy("ios", "Xcode 25.0", "26.0", "Xcode.app").is_err());
        assert!(apple_sdk_policy("ios", "Xcode 26.4", "18.0", "Xcode.app").is_err());
        assert!(apple_sdk_policy("ios", "Xcode 27.1", "27.1", "Xcode-beta.app").is_err());
    }
}
