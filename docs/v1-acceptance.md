# v1 Acceptance

Preside v1 is complete when a user can install the macOS app, add either a static folder or a server command, run and debug it, edit/restart/rename/remove it, and recover from ordinary failures without using a terminal or losing control of a server.

## Evidence recorded 2026-11-04

- Rust checks: format, Clippy with warnings denied, 52 tests pass. Coverage includes concurrent mutations/starts, whole-site update conflicts, startup rollback on persistence failure, early process exit, missing PID recovery, unknown-process refusal, malformed-config stop, static HTTP serving, and traversal rejection.
- Swift checks: format and 26 tests pass. Coverage includes client timeouts/argument handling, action serialization, error persistence, whole-site lifecycle through the real CLI, and multiline environment preservation.
- Packaged development app: ZIP/DMG/checksums generated; ZIP extracted into a separate path containing spaces and launched with isolated HOME and a minimal PATH. GUI edit, start, restart, stop, rename, and removal passed for a static site. HTTP returned the edited folder's content. Removal preserved project files and left the port closed.
- Earlier GUI checks exercised creation/start/stop and HTTP responses for both static and command-based sites.
- Release scripts reject missing signing/notarization inputs. The current machine has no Developer ID Application signing identity; distribution execution is not verified.

## Remaining release gates

- [ ] Close or otherwise explicitly resolve the parent-death window between spawning a service and recording its initial runtime state. Error-path rollback is tested; abrupt death at every instruction boundary is not covered.
- [ ] Verify child-process cleanup when a command's original process exits before its descendants.
- [ ] Expose configured readiness and actionable starting/ready/unhealthy/failed state.
- [ ] Provide expanded/live logs, copy/export, and an ongoing log-growth policy.
- [ ] Suggest available ports and assist with missing project tools/Finder PATH differences.
- [ ] Define direct-domain versus proxied URL behavior; the current domain URL assumes an external proxy.
- [ ] Finish native UI acceptance for menu-bar-only workflows, keyboard operation, VoiceOver, moved folders, failures, and app relaunch.
- [ ] Run distribution signing, notarization, stapling, and Gatekeeper assessment with real release credentials.
- [ ] Download/install the actual release on a clean Apple Silicon Mac without developer tools, including the minimum supported macOS version.
- [ ] Verify upgrading an installed older app preserves settings and control of sites.
- [ ] Confirm hosted CI for the final release commit, then publish the versioned app and CLI artifacts.

The app uses manual downloads from GitHub Releases for updates. Automatic DNS/HTTPS, a daemon/supervisor, automatic server startup, Docker management, and cloud sync are outside this v1 scope. SPA fallback and launching the app at login remain optional enhancements.

Local ad-hoc packaging, distribution signing, notarization, and clean-machine acceptance are separate evidence levels. Passing one does not imply the next.
