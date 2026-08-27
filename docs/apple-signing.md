# Apple signing with JFFI

Apple distribution requires a signing identity, a team, entitlements, and a
provisioning profile for every signed bundle identifier in the archive. JFFI
keeps the non-secret mapping in `jffi.toml` and relies on Xcode and the macOS
keychain for the actual signing material.

## Single-target application

An application with no signed extensions can use the legacy singular profile:

```toml
[bundle.signing]
profile = "ios-appstore"

[bundle.signing.profiles.ios-appstore.apple]
team_id = "YOUR_TEAM_ID"
method = "app-store-connect"
signing_certificate = "Apple Distribution"
provisioning_profile = "My App Store Profile"
```

The profile name must match an installed provisioning profile for the
application's bundle identifier.

## Application with extensions

Every extension is a separately signed target. Map the main application and
each extension bundle identifier to its own profile:

```toml
[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.myapp"
app_groups = ["group.com.example.myapp"]

[bundle.signing]
profile = "ios-appstore"

[bundle.signing.profiles.ios-appstore.apple]
team_id = "YOUR_TEAM_ID"
method = "app-store-connect"
signing_certificate = "Apple Distribution"

[bundle.signing.profiles.ios-appstore.apple.provisioning_profiles]
"com.example.myapp" = "My App Store Profile"
"com.example.myapp.ShareExtension" = "My Share Extension Profile"
```

JFFI writes this mapping into the iOS export options while retaining the
target-specific signing settings in the Xcode archive. Do not assign the main
application profile to an extension: its application identifier and
entitlements will not match.

`provisioning_profile` remains supported for a single-target application. If a
non-empty `provisioning_profiles` mapping is also present, the mapping is the
authoritative multi-target configuration.

## Capabilities and entitlements

The application identifier, App ID capabilities, entitlements file, and
provisioning profile must agree. This is especially important for application
groups, push notifications, associated domains, iCloud, and extensions.

For an application group:

1. Register the group in the Apple Developer account.
2. Enable the capability for every App ID that uses it.
3. Regenerate the affected provisioning profiles.
4. Install the profiles on the build host or import them in CI.
5. Keep the same group identifier in `jffi.toml` and each target's entitlements.

JFFI cannot repair an expired certificate, a revoked profile, an unaccepted
Apple agreement, or a capability missing from the Apple Developer account.

## Notarizing macOS applications

For macOS, configure signing and keep the credentials outside source control.
JFFI supports a `notarytool` keychain profile through
`JFFI_APPLE_NOTARY_PROFILE`. A signing profile can also reference environment
variable names or App Store Connect API-key metadata rather than embedding
secrets.

```toml
[bundle.signing.profiles.release.apple]
team_id = "YOUR_TEAM_ID"
signing_certificate = "Developer ID Application"
installer_signing_certificate = "Developer ID Installer"
notarize = true
```

```bash
export JFFI_APPLE_NOTARY_PROFILE="my-notary-profile"
jffi bundle --platform macos --profile release --notarize
```

Create the keychain profile ahead of time with Apple's `notarytool`. Do not put
an Apple ID password or app-specific password directly in `jffi.toml`.

## Release preflight

Run the checks on a macOS host with the intended Xcode version and signing
assets installed:

```bash
jffi doctor config
jffi doctor bundle --platform ios --release --profile ios-appstore
jffi bundle --platform ios --profile ios-appstore --dry-run --print-plan
```

For macOS, repeat with `--platform macos` and the matching profile. Before
uploading, inspect the archive and verify that every nested executable is signed
by the expected team and that its entitlements match its provisioning profile.

## Common failures

| Failure | Likely cause |
| --- | --- |
| A profile does not include the signing certificate | The certificate was renewed after the profile was created |
| Application identifier entitlement mismatch | A profile belongs to a different bundle identifier |
| Extension cannot be signed | The extension has no dedicated profile mapping |
| Application group entitlement is missing | The capability or regenerated profile is missing for one target |
| Export succeeds locally but fails in CI | CI did not import the same certificate, private key, or profiles |
| Notarization authentication fails | The keychain profile or App Store Connect credentials are unavailable to the job |

Treat a successful archive, export, signature verification, and store upload as
separate release gates. Passing one does not prove that the later gates will
succeed.
