# v1 Acceptance

Preside v1 is complete when a user can install the macOS app, add either a static folder or a server command, run and debug it, edit/restart/rename/remove it, and recover from ordinary failures without using a terminal or losing control of a server.

## Evidence recorded 2026-11-04

- Rust checks: format, Clippy with warnings denied, 60 tests pass. Coverage includes concurrent mutations/starts, whole-site update conflicts, startup rollback on persistence failure, early process exit, missing PID recovery, unknown-process refusal, malformed-config stop, static HTTP serving, and traversal rejection.
- Swift checks: format and 37 tests pass. Coverage includes client timeouts/argument handling, action serialization, error persistence, whole-site lifecycle through the real CLI, and multiline environment preservation.
- Packaged development app: ZIP/DMG/checksums generated; ZIP extracted into a separate path containing spaces and launched with isolated HOME and a minimal PATH. GUI edit, start, restart, stop, rename, and removal passed for a static site. HTTP returned the edited folder's content. Removal preserved project files and left the port closed.
- Earlier GUI checks exercised creation/start/stop and HTTP responses for both static and command-based sites.
- Release scripts reject missing signing/notarization inputs. The current machine has no Developer ID Application signing identity; distribution execution is not verified.

- Readiness and setup: Rust readiness probes, explicit direct/proxy URLs, and bounded JSON log tails pass tests. Swift tests cover status decoding, live-log recovery and cross-site request races, port suggestions, PATH lookup, and URL-mode compatibility. Existing domain configurations preserve their legacy URL behavior.
- Launch ownership: a gated exec helper cannot run the service until initial runtime state and PID are durable. Deterministic controller-death tests cover before publication, after publication, partial release, and completed release; ordinary CLI tests cover integrated startup failure and recovery.
- Native smoke check: the built app displayed Ready for a serving static site, opened expanded logs, automatically displayed newly appended output, copied that output into a disposable form, and exported the expected text through the native Save dialog. The rebuilt viewer was visually verified to open at the left edge and expose log content in the accessibility tree. Add Site reopened by keyboard with the main window closed; Suggest Port populated a port, and entering a domain displayed the direct URL with that port. A status-menu keyboard attempt timed out in the UI-control tool, so direct menu-bar acceptance remains unverified.

- Failure and recovery smoke: GUI Start displayed missing-command and occupied-port errors without starting conflicting services. A missing folder was repaired with Edit Site, then served the expected HTTP content. The site continued serving after app quit, was rediscovered as Ready with the same PID after relaunch, and stopped through the GUI with its port closed. Switching site selection revealed a stale success notice; the notice is now scoped to its target site, covered by regression tests and verified in the rebuilt UI.

- Local upgrade regression: `CARGO_PROFILE_RELEASE_STRIP=none ./scripts/test-macos-upgrade.sh 0658a87` passed from the prior app to the 80ec303-based bundle. Static and command sites kept identical configuration bytes, PIDs, verified identities, and working HTTP responses across replacement. The new CLI stopped both sites and ports closed. Current CLI SHA-256 was `8999ff8f985510fda4c48042f72e02d4830d4d729c8893b3d1be208de086023c`. The unstripped build override worked around this host's linker rejection of a stripped proc-macro dylib; no production code workaround was applied. The check uses preflight-verified isolated HOME/XDG paths and a minimal PATH. GUI preferences and signed/quarantined upgrade behavior are not covered.

- Native preference upgrade: saved a temporary PATH prefix in the prior extracted app, quit it, replaced its bundle at the same installation path with the current app, and verified that Settings retained the prefix. Restored and verified the original PATH afterward. The bundled-CLI selection remained empty across replacement. This is local preference continuity, not signed/quarantined upgrade acceptance.
- Keyboard/accessibility inspection: Command-comma opened Settings; Command-N opened Add Site; Tab moved from Name to Project folder; Escape cancelled the sheet. The native tool lookup resolved `/usr/bin/true`. VoiceOver was temporarily enabled and restored to its original off state, but the UI-control surface did not establish spoken output or VoiceOver navigation. That acceptance gate remains open.

## Remaining release gates

- [x] Gate service execution on durable runtime publication; test controller death at launch handoff boundaries.
- [ ] Verify child-process cleanup when a command's original process exits before its descendants.
- [x] Expose configured readiness and actionable starting/ready/unhealthy/failed state.
- [x] Provide expanded/live logs with copy/export controls.
- [ ] Bound ongoing log growth; prelaunch rotation does not cap a continuously running service. Finish VoiceOver acceptance.
- [x] Suggest available ports and assist with missing project tools/Finder PATH differences.
- [x] Define explicit direct-domain versus proxied URL behavior while preserving existing domain configurations.
- [x] Verify native moved-folder recovery, missing-command/occupied-port errors, and app relaunch without losing server control.
- [ ] Finish native UI acceptance for menu-bar-only workflows, full keyboard operation, and VoiceOver.
- [ ] Run distribution signing, notarization, stapling, and Gatekeeper assessment with real release credentials.
- [ ] Download/install the actual release on a clean Apple Silicon Mac without developer tools, including the minimum supported macOS version.
- [x] Verify local app replacement preserves site configuration and control of running static/command sites.
- [x] Verify local PATH preference continuity and unchanged bundled-CLI selection across app replacement.
- [ ] Verify signed/quarantined upgrades of the distributed app.
- [ ] Confirm hosted CI for the final release commit, then publish the versioned app and CLI artifacts.

The app uses manual downloads from GitHub Releases for updates. Automatic DNS/HTTPS, a daemon/supervisor, automatic server startup, Docker management, and cloud sync are outside this v1 scope. SPA fallback and launching the app at login remain optional enhancements.

Local ad-hoc packaging, distribution signing, notarization, and clean-machine acceptance are separate evidence levels. Passing one does not imply the next.
