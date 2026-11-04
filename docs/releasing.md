# Releasing Preside

The first distributed macOS app targets Apple Silicon and macOS 14 or later. Intel builds remain a local development option; the release workflow does not claim universal support. The Rust CLI is also packaged for Linux x86_64.

`Cargo.toml` is the app's marketing version source. The build script writes that version into the app bundle, and release tags must match `v<version>`. `PRESIDE_BUILD_NUMBER` controls `CFBundleVersion`; local builds default to 1 and GitHub releases use the workflow run number.

## Local development packages

```sh
./scripts/package-macos.sh
```

This builds `dist/Preside.app` with its bundled `pp`, signs it ad hoc, and creates an explicitly named development ZIP, DMG, and SHA-256 manifest. These artifacts are for local validation. They are not Developer ID signed or notarized and are not suitable for a public download claiming Gatekeeper acceptance.

The DMG contains Preside and an Applications shortcut. The ZIP contains the application bundle. Installing either package does not require Rust, Xcode, Node, or Python; command-based sites still require their own project tools.

## Signed distribution packages

Prerequisites:

- A Developer ID Application certificate and its private key in an unlocked keychain.
- A `notarytool` keychain profile with valid Apple notarization credentials.
- An Apple Silicon build Mac with Rust and Xcode installed.

Set `PRESIDE_SIGNING_IDENTITY` to the certificate's full name or fingerprint and `PRESIDE_NOTARY_PROFILE` to the profile name. If the identity and profile are in a non-default keychain, also set `PRESIDE_SIGNING_KEYCHAIN` to its path.

```sh
./scripts/package-macos.sh --distribution
```

Distribution mode fails when credentials are missing. It signs both `pp` and the app using the hardened runtime and timestamp, verifies a Developer ID Application authority, submits the app for notarization, checks the acceptance result, staples the ticket, and assesses the app with Gatekeeper. It then creates the ZIP and DMG, signs/notarizes/staples the DMG, and assesses the DMG. Checksums are generated only after these steps succeed.

The app inside the distribution ZIP carries a stapled ticket. The DMG carries its own ticket in addition to the enclosed app's ticket. A failed signing or notarization operation never becomes a successful development fallback.

## GitHub release configuration

The tag-triggered release workflow requires a GitHub environment named `release`. Configure these secrets in that environment:

| Secret | Value |
| --- | --- |
| `MACOS_CERTIFICATE_BASE64` | Base64-encoded PKCS#12 containing the Developer ID Application certificate and private key |
| `MACOS_CERTIFICATE_PASSWORD` | Password protecting that PKCS#12 |
| `MACOS_SIGNING_IDENTITY` | Developer ID Application certificate name or fingerprint |
| `APPLE_API_KEY_BASE64` | Base64-encoded App Store Connect API `.p8` key authorized for notarization |
| `APPLE_API_KEY_ID` | API key ID |
| `APPLE_API_ISSUER` | API issuer ID |

The workflow imports credentials into a temporary keychain and removes them in an always-run cleanup step. No credentials belong in this repository. The release publication job depends on successful CLI and notarized-app builds; missing credentials block publication rather than producing a CLI-only release silently.

After reviewing changes, passing CI, and updating the version, create and push the matching version tag. Tagging publishes release assets and is a separate release action from pushing a development branch.

## Acceptance before public release

- Download the actual release ZIP and DMG in a browser so normal quarantine behavior applies.
- Install and launch on a clean Apple Silicon Mac without development tools; verify no bypass of Gatekeeper is needed.
- Add, run, edit, restart, and remove both site types; verify logs, errors, port conflicts, moved folders, and relaunch.
- Exercise menu-bar-only usage, keyboard navigation, and VoiceOver.
- Verify the documented minimum macOS version on that OS, not only the build Mac.
- Verify replacement of an older installed app preserves configuration and control of running sites.

Local ad-hoc packaging tests, code-signing verification, notarization, and clean-machine acceptance are distinct gates. Do not claim the later gates from earlier ones.

## Manual updates

Settings > Download Updates opens the latest GitHub Release. Download the appropriate app archive or DMG, quit Preside, replace the installed application, and reopen it. Configuration and runtime state live outside the app bundle. Validate continuity with existing sites before publishing an update; automatic installation and rollback are not implemented.
