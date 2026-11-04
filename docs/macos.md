# macOS App

Preside includes a native SwiftUI app for macOS 14 and later. It shares the CLI's configuration and runtime state; services started in a terminal appear in the app, and detached services continue running when the app quits.

## Build and Run

Install Rust and Xcode with Swift 6 or later, then run:

```sh
just macos-build
open dist/Preside.app
```

The build creates an ad-hoc signed app containing both the Swift executable and a release build of `pp`. It builds for the current Mac's architecture. You can move the app to Applications. Distribution signing, notarization, universal binaries, and automatic updates are not configured.

Open `macos/Package.swift` in Xcode to develop the interface and use its SwiftUI previews. When running the package directly, install `pp` or select a built executable in Settings. The packaged app uses its bundled `pp` by default.

## Features

- Browse and search configured services.
- Start detached services, stop verified Preside processes, and open configured URLs.
- Inspect configuration, PID, TCP listening status, and the five most recent log lines; reveal the full log in Finder.
- Run diagnostics without automatically applying repairs.
- Add sites directly from the menu bar or main window. Choose **Static folder** to serve a directory with the built-in server, or **Server command** to launch a development server such as `npm run dev`.
- Configure a port and optional domain for either type; command sites also support readiness checks and environment variables.
- Access services from the menu bar, including after closing the main window.
- Refresh automatically every three seconds after the previous refresh finishes, or manually with Command-R.

Use the terminal icon in the macOS menu bar to choose **Add Site…**, start or stop a site, open its URL, or show details. Closing the main window leaves the menu bar available.

Static sites serve the selected folder on `127.0.0.1`, including `index.html` and assets, without requiring Node or Python. Choose your built output folder for projects that need a build step.

Commands run with your account's privileges. Adding a site saves its definition; starting a command site executes the configured shell command. The app delegates all service actions to `pp` using argument arrays. It does not install a daemon, configure DNS or TLS, or change login items.

The interface distinguishes a running process from a listening port. It does not claim that HTTP health checks are passing. An unknown live process cannot be stopped through the interface. Older runtime state without a recorded process start marker is treated as unknown.

## Configuration and PATH

Settings can override the `pp` executable and the PATH used by service commands. A blank executable setting chooses the bundled CLI, falling back to PATH when running the Swift package directly. Default PATH includes Homebrew, `~/.cargo/bin`, `~/.local/bin`, and the app's inherited PATH.

Finder does not load interactive shell configuration. For version managers such as nvm, specify the directory containing your chosen Node executable in PATH, or use an absolute executable path in the service command. The app does not run shell startup files automatically.

Use More > Reveal Configuration in Finder to locate the existing TOML file for edits. Full service editing, removal, and renaming are intentionally not exposed yet: the current CLI does not provide atomic multi-field updates, and simultaneous app/terminal config changes can overwrite each other. Avoid simultaneous configuration changes or starts from multiple clients.

Custom domains require external DNS/proxy setup, just as they do in the CLI.

## Validation

```sh
just check
just macos-format-check
PRESIDE_TEST_CLI="$PWD/target/release/pp" just macos-test
```

Build the app first for the release CLI integration test. Swift tests use temporary files and an isolated HOME for service lifecycle checks. Without `PRESIDE_TEST_CLI`, the real CLI lifecycle test is skipped. The test script builds in temporary storage to avoid Finder metadata interfering with test-bundle signing in synced directories.

The GitHub Actions macOS job checks Swift formatting, builds the bundle, and runs tests with the bundled CLI's release binary. This is separate from Rust's Linux/macOS checks.
