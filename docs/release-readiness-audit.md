# Shared release tooling audit — November 10, 2026

This audit covers JFFI's build machinery. It does not certify SEYFR's signed
release candidates, store declarations, or physical-device behavior.

## Findings and changes

| Area | Finding | Resolution |
| --- | --- | --- |
| Android architectures | x86_64 already exists, but unknown ABIs could be silently filtered out and the Rust/bundle lists could disagree | One resolver validates supported, nonempty, unique, matching ABI sets and drives both Rust builds and bundling. Bundle-only ABI selection now reaches the Rust build too. |
| Windows architectures | x64, ARM64 and x86 already exist | No new backend needed. Fixed setup rejecting the existing explicit x86 build target while preserving x64/ARM64 defaults. Configuration preflight now rejects unknown, empty and duplicate target selections before native work. Existing architecture-specific staging regression coverage remains. |
| Android SDKs | compileSdk was not synchronized alongside targetSdk/minSdk | Synchronize all three and reject min > target or target > compile. Warn about the current Play API 36 submission floor without preventing non-Play builds for older targets. |
| Native compatibility | Linker flags and signatures alone did not establish final package compatibility | Inspect APK/AAB native payloads: core presence in each configured ABI, no unconfigured ABIs, ELF architecture/type, and every 64-bit LOAD segment's 16 KB alignment/congruence. Dependencies and AAB feature-module libraries are included. |
| Android ZIP alignment | No final APK/AAB alignment gate | Run SDK zipalign on APKs. Read AAB BundleConfig.pb and reject uncompressed native libraries requesting default/4 KB ZIP alignment. Compressed libraries still require valid ELF alignment. |
| Apple SDKs | Doctor checked that Xcode existed, without a store SDK floor | Signed App Store exports and release doctor preflight check selected Xcode/device SDK, honoring DEVELOPER_DIR. Labeled beta/RC paths are rejected. The current iOS floor is Xcode/SDK 26; SDK 27 becomes mandatory in April 2027. Development/website exports retain their existing toolchain flexibility. |
| iOS archive overrides | A project could use an older/simulator SDK despite a current selected toolchain | Inspect DTSDKName in the archived app and embedded extensions. SDK 27 app bundles require launch-screen metadata. This catches SDKROOT overrides before IPA export. |
| Signing | APK/AAB, Apple archive/app/PKG and Windows signing verification already exist | Retained existing checks. New native compatibility gates run even for unsigned Android release bundles. |
| Linux | Flatpak runtime/SDK reconciliation and package generation already exist | No extra device-family backend needed. Host architecture, sandbox acceptance and distribution availability must still be verified for each app. |

## Adversarial review corrections

- Preserve whether bundle ABI/minimum-SDK values were explicitly configured.
  Omitted values inherit platform settings, including after config serialization.
  Explicit empty/duplicate lists and conflicting values still fail.
- Reject BundleConfig.pb larger than 1 MiB; never validate a silently truncated
  prefix that could omit the native alignment settings.
- Inspect native libraries only immediately under a module's lib directory.
  Application assets, BUNDLE-METADATA and META-INF entries are excluded; native
  libraries in real feature-module lib directories remain checked.
- Accept all four Apple launch-screen keys, including UILaunchScreens.
- Apply generated ABI filters through Android Components finalizeDsl, after the
  app DSL is evaluated. Loading JFFI's script before android {} no longer lets
  the template's default ABI list add excluded architectures back. The Android
  CI job now builds an ARM64-only release APK with JNA and checks its actual ZIP
  payload; the strict artifact validator remains enabled. The finalization hook
  normalizes defaultConfig, build types and product flavors. CI deliberately adds
  an x86_64 release-build override to exercise that merge boundary.
- Honor JAVA_HOME for Gradle wrapper bootstrapping in bundle/run and for Android
  doctor Java checks. A selected JDK no longer loses to a different Java on PATH.
  Subprocess regression coverage boots a missing wrapper using a JDK path with
  spaces while PATH contains an intentionally failing Java.

## Reusing the artifact check

Run from the JFFI application project, using its ABI configuration and
core/Cargo.toml library name:

```sh
jffi --no-setup doctor artifact --platform android --artifact path/to/app.aab
jffi --no-setup doctor artifact --platform android --artifact path/to/app.apk
```

This command checks native payload/16 KB compatibility. It does not verify
signatures, all Android manifest metadata, bundletool acceptance, store policy,
or runtime behavior. Actual bundling retains its existing signature checks.
APK inspection needs SDK zipalign; JFFI_ZIPALIGN can select it explicitly.
AAB inspection does not require an Android SDK installation.

## Validation and remaining boundaries

Regression coverage includes ABI conflicts/typos/duplicates, compile SDK sync,
wrong ELF architecture, malformed headers, incompatible third-party libraries,
missing core payloads, AAB native ZIP settings, Apple SDK floors, archive SDK
and extension overrides, launch-screen metadata, and the public artifact doctor.
The new command also passed against SEYFR's existing local release AAB and debug
APK. Those are compatibility checks of existing artifacts, not certification of a
fresh signed release. The complete existing CLI suite and strict Clippy passed
locally; Apple archive metadata integration uses the real macOS plutil tool.

No framework switch establishes foldable hinge handling, state retention through
resizing, camera availability, background transfer behavior, accessibility,
ARM tablet acceptance, or original-file integrity. Those remain app/device QA.
Final signed Apple/Windows packages, provisioning and entitlement correctness,
privacy-reason accuracy, store-side declarations/trader status, screenshots and
Windows App Certification Kit remain separate release gates. JFFI does not
change SEYFR's selected shipping architectures or submit/publish anything.

## Sources

- [Apple current submission requirements](https://developer.apple.com/news/upcoming-requirements/)
- [Apple SDK 27 and Duo submission guidance](https://developer.apple.com/app-store/submitting/)
- [Apple launch-screen requirement](https://developer.apple.com/documentation/technotes/tn3208-preparing-your-apps-launch-screen-to-meet-app-store-requirements)
- [Android 16 KB page sizes](https://developer.android.com/guide/practices/page-sizes)
- [Google Play target API policy](https://support.google.com/googleplay/android-developer/answer/11926878)
- [Bundletool's native alignment schema](https://github.com/google/bundletool/blob/master/src/main/proto/config.proto)
