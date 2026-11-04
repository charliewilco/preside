import Foundation
import Testing

@testable import PresideApp

@MainActor
struct ServiceStoreTests {
  @Test func failedActionRemainsVisibleAfterSuccessfulRefreshUntilDismissed() async throws {
    let fixture = try StoreFixture(
      """
      case "$1" in
        list) printf '{}' ;;
        run) printf 'Service could not start' >&2; exit 1 ;;
        *) exit 2 ;;
      esac
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)

    await store.perform(.start, name: "example")
    #expect(store.error == "Service could not start")
    #expect(!store.isBusy)

    await store.refresh()
    #expect(store.error == "Service could not start")
    #expect(!store.isRefreshing)

    store.error = nil
    await store.refresh()
    #expect(store.error == nil)
  }

  @Test func actionWaitsForRefreshInsteadOfBeingDropped() async throws {
    let fixture = try StoreFixture(
      """
      directory="$(dirname "$0")"
      case "$1" in
        list)
          touch "$directory/refresh-started"
          attempts=0
          while [ ! -f "$directory/release-refresh" ] && [ "$attempts" -lt 250 ]; do
            /bin/sleep 0.02
            attempts=$((attempts + 1))
          done
          printf '{}'
          ;;
        run) touch "$directory/action-ran"; printf 'Started example' ;;
        *) exit 2 ;;
      esac
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    let refresh = Task { await store.refresh() }
    let started = fixture.directory.appendingPathComponent("refresh-started")
    let startDeadline = ContinuousClock.now.advanced(by: .seconds(3))
    while !FileManager.default.fileExists(atPath: started.path)
      && ContinuousClock.now < startDeadline
    {
      try await Task.sleep(for: .milliseconds(10))
    }
    #expect(store.isRefreshing)
    #expect(FileManager.default.fileExists(atPath: started.path))

    let action = Task { await store.perform(.start, name: "example") }
    let actionDeadline = ContinuousClock.now.advanced(by: .seconds(2))
    while !store.isBusy && ContinuousClock.now < actionDeadline {
      try await Task.sleep(for: .milliseconds(10))
    }
    #expect(store.isBusy)
    #expect(store.isRefreshing)
    try Data().write(to: fixture.directory.appendingPathComponent("release-refresh"))
    await refresh.value
    await action.value

    #expect(
      FileManager.default.fileExists(
        atPath: fixture.directory.appendingPathComponent("action-ran").path))
    #expect(store.notice == "Started example")
    #expect(store.error == nil)
    #expect(!store.isBusy)
    #expect(!store.isRefreshing)
  }

  @Test func recoveredRefreshClearsOnlyItsOwnError() async throws {
    let fixture = try StoreFixture("printf 'Cannot read configuration' >&2; exit 1")
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)

    await store.refresh()
    #expect(store.error == "Cannot read configuration")

    try fixture.write("printf '{}'")
    await store.refresh()
    #expect(store.error == nil)
    #expect(store.services.isEmpty)
  }

  @Test func failedDiagnosticsRemainVisibleAfterSuccessfulRefresh() async throws {
    let fixture = try StoreFixture(
      """
      case "$1" in
        list) printf '{}' ;;
        doctor) printf 'Cannot diagnose service' >&2; exit 2 ;;
        *) exit 2 ;;
      esac
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)

    await store.diagnose(name: "example")
    #expect(store.error == "Cannot diagnose service")
    #expect(store.doctorOutput == nil)

    await store.refresh()
    #expect(store.error == "Cannot diagnose service")
  }
  @Test func logRefreshPreservesOutputOnFailureAndRecovers() async throws {
    let fixture = try StoreFixture(
      "printf '%s' '{\"path\":\"/tmp/test.log\",\"lines\":[\"first\",\"second\"]}'")
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    store.logSiteName = "example"
    await store.refreshLogs(name: "example")
    #expect(store.logOutput == "first\nsecond")
    #expect(store.logError == nil)
    #expect(!store.isLoadingLogs)
    try fixture.write("printf 'Cannot read logs' >&2; exit 1")
    await store.refreshLogs(name: "example")
    #expect(store.logOutput == "first\nsecond")
    #expect(store.logError == "Cannot read logs")
    #expect(!store.isLoadingLogs)
    try fixture.write("printf '%s' '{\"path\":\"/tmp/test.log\",\"lines\":[\"recovered\"]}'")
    await store.refreshLogs(name: "example")
    #expect(store.logOutput == "recovered")
    #expect(store.logError == nil)
  }

  @Test(arguments: [false, true])
  func lateLogResponseCannotReplaceAnotherSitesOutput(fails: Bool) async throws {
    let fixture = try StoreFixture(
      """
      directory="$(dirname "$0")"
      touch "$directory/started"
      attempts=0
      while [ ! -f "$directory/release" ] && [ "$attempts" -lt 250 ]; do
        /bin/sleep 0.02
        attempts=$((attempts + 1))
      done
      \(fails ? "printf 'Old failure' >&2; exit 1" : "printf '%s' '{\"path\":\"/tmp/test.log\",\"lines\":[\"old output\"]}'")
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    store.logSiteName = "old"
    let request = Task { await store.refreshLogs(name: "old") }
    let started = fixture.directory.appendingPathComponent("started")
    let deadline = ContinuousClock.now.advanced(by: .seconds(3))
    while !FileManager.default.fileExists(atPath: started.path) && ContinuousClock.now < deadline {
      try await Task.sleep(for: .milliseconds(10))
    }
    #expect(FileManager.default.fileExists(atPath: started.path))
    store.logSiteName = "new"
    store.logOutput = "New site output"
    store.logError = "New site warning"
    try Data().write(to: fixture.directory.appendingPathComponent("release"))
    await request.value
    #expect(store.logOutput == "New site output")
    #expect(store.logError == "New site warning")
    #expect(!store.isLoadingLogs)
  }

  @Test func newSiteLogsLoadWhilePreviousRequestIsPending() async throws {
    let fixture = try StoreFixture(
      """
      directory="$(dirname "$0")"
      if [ "$6" = "old" ]; then
        touch "$directory/started"
        attempts=0
        while [ ! -f "$directory/release" ] && [ "$attempts" -lt 250 ]; do
          /bin/sleep 0.02
          attempts=$((attempts + 1))
        done
      fi
      printf '{"path":"/tmp/test.log","lines":["%s"]}' "$6"
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    store.logSiteName = "old"
    let oldRequest = Task { await store.refreshLogs(name: "old") }
    let started = fixture.directory.appendingPathComponent("started")
    let deadline = ContinuousClock.now.advanced(by: .seconds(3))
    while !FileManager.default.fileExists(atPath: started.path) && ContinuousClock.now < deadline {
      try await Task.sleep(for: .milliseconds(10))
    }
    #expect(FileManager.default.fileExists(atPath: started.path))
    store.logSiteName = "new"
    await store.refreshLogs(name: "new")
    #expect(store.logOutput == "new")
    #expect(store.logError == nil)
    #expect(!store.isLoadingLogs)
    try Data().write(to: fixture.directory.appendingPathComponent("release"))
    await oldRequest.value
    #expect(store.logOutput == "new")
    #expect(!store.isLoadingLogs)
  }

  @Test func actionNoticeStaysWithTargetAcrossSelectionAndPolling() async throws {
    let fixture = try StoreFixture(
      """
      case "$1" in
        list) printf '{"started":{},"other":{}}' ;;
        status) printf '{"status":{"process_alive":false,"identity_matches":false,"recent_log_lines":[],"log_path":"/tmp/test.log"},"missing_fields":[]}' ;;
        run) printf 'Started target site' ;;
        stop) printf 'Stop failed' >&2; exit 1 ;;
      esac
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    store.selection = "other"
    await store.perform(.start, name: "started")
    #expect(store.selection == "other")
    #expect(store.notice(for: "other") == nil)
    #expect(store.notice(for: "started") == "Started target site")
    store.selection = "started"
    await store.refresh()
    #expect(store.notice(for: "started") == "Started target site")
    store.selection = "other"
    await store.refresh()
    #expect(store.notice(for: "other") == nil)
    #expect(store.notice(for: "started") == "Started target site")
    await store.perform(.stop, name: "started")
    #expect(store.notice(for: "started") == nil)
  }

  @Test func addRenameAndRemoveNoticesFollowResultingSite() async throws {
    let fixture = try StoreFixture(
      """
      case "$1" in
        list) printf '{"added":{},"renamed":{},"other":{}}' ;;
        status) printf '{"status":{"process_alive":false,"identity_matches":false,"recent_log_lines":[],"log_path":"/tmp/test.log"},"missing_fields":[]}' ;;
        add) printf 'Added site' ;;
        rename) printf 'Renamed site' ;;
        remove) printf 'Removed site' ;;
      esac
      """)
    defer { fixture.remove() }
    let store = ServiceStore(defaults: fixture.defaults)
    store.selection = "other"
    let draft = ServiceDraft(name: "added", dir: "/tmp/site", command: "run", port: "8123")
    #expect(await store.add(draft))
    #expect(store.selection == "added")
    #expect(store.notice(for: "added") == "Added site")
    #expect(store.notice(for: "other") == nil)
    #expect(await store.rename(name: "added", to: "renamed"))
    #expect(store.selection == "renamed")
    #expect(store.notice(for: "renamed") == "Renamed site")
    #expect(store.notice(for: "added") == nil)
    #expect(store.notice(for: "other") == nil)
    await store.refresh()
    #expect(store.notice(for: "renamed") == "Renamed site")
    store.selection = "other"
    #expect(await store.remove(name: "renamed"))
    #expect(store.selection == "other")
    #expect(store.notice(for: "other") == nil)
  }

}

private struct StoreFixture {
  let directory: URL
  let executable: URL
  let suiteName: String
  let defaults: UserDefaults

  init(_ body: String) throws {
    directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    executable = directory.appendingPathComponent("pp-fixture")
    suiteName = "PresideStoreTests.\(UUID().uuidString)"
    defaults = try #require(UserDefaults(suiteName: suiteName))
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try write(body)
    defaults.set(executable.path, forKey: "executablePath")
  }

  func write(_ body: String) throws {
    try Data("#!/bin/sh\n\(body)\n".utf8).write(to: executable)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: executable.path)
  }

  func remove() {
    defaults.removePersistentDomain(forName: suiteName)
    try? FileManager.default.removeItem(at: directory)
  }
}
