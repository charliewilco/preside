import Foundation
import Testing

@testable import PresideApp

struct CLIClientTests {
  @Test func acceptedDiagnosticExitPreservesJSONAndStderr() async throws {
    let fixture = try ScriptFixture(
      "printf '%s' '{\"issues\":1}'\nprintf '%s' 'diagnostic' >&2\nexit 1")
    defer { fixture.remove() }
    let client = CLIClient(executableURL: fixture.executable)
    let result = try await client.run([], acceptedExitCodes: [0, 1])
    #expect(result.exitCode == 1)
    #expect(result.output == #"{"issues":1}"#)
    #expect(result.diagnostic == "diagnostic")
    do {
      _ = try await client.run([])
      Issue.record("An unaccepted exit code must throw")
    } catch {
      #expect(error.localizedDescription == "diagnostic")
    }
  }

  @Test func largeStdoutAndStderrCompleteWithoutDeadlock() async throws {
    let fixture = try ScriptFixture(
      """
      /usr/bin/awk 'BEGIN { for (i=0; i<20000; i++) print "abcdefgh" }'
      /usr/bin/awk 'BEGIN { for (i=0; i<20000; i++) print "ijklmnop" }' >&2
      """)
    defer { fixture.remove() }
    let result = try await CLIClient(executableURL: fixture.executable, timeout: 10).run([])
    #expect(result.output.split(separator: "\n").count == 20000)
    #expect(result.diagnostic.split(separator: "\n").count == 20000)
    #expect(result.output.hasPrefix("abcdefgh"))
    #expect(result.diagnostic.hasSuffix("ijklmnop"))
  }

  @Test func timeoutTerminatesCommandAndReturnsActionableError() async throws {
    let fixture = try ScriptFixture("exec /bin/sleep 10")
    defer { fixture.remove() }
    let start = ContinuousClock.now
    do {
      _ = try await CLIClient(executableURL: fixture.executable, timeout: 0.1).run([])
      Issue.record("A command exceeding its deadline must throw")
    } catch {
      #expect(error.localizedDescription.contains("timed out"))
      #expect(error.localizedDescription.contains("refresh"))
    }
    #expect(start.duration(to: .now) < .seconds(5))
  }

  @Test func argumentsArePassedWithoutShellEvaluation() async throws {
    let fixture = try ScriptFixture("printf '%s\\n' \"$@\"")
    defer { fixture.remove() }
    let input = ["space separated", "$(printf injected)", "; echo injected", "a=b=c"]
    let result = try await CLIClient(executableURL: fixture.executable).run(input)
    #expect(result.output == input.joined(separator: "\n"))
  }

  @Test(.enabled(if: ProcessInfo.processInfo.environment["PRESIDE_TEST_CLI"] != nil))
  func realCLILifecycleUsesIsolatedHome() async throws {
    let executable = try #require(ProcessInfo.processInfo.environment["PRESIDE_TEST_CLI"])
    let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: home) }
    var environment = ProcessInfo.processInfo.environment
    environment["HOME"] = home.path
    let client = CLIClient(
      executableURL: URL(fileURLWithPath: executable), environment: environment, timeout: 10)
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let draft = ServiceDraft(
      name: "gui-integration", dir: home.path, command: "exec /bin/sleep 60", port: "54321",
      readiness: "process"
    )
    _ = try await client.run(draft.arguments)
    do {
      _ = try await client.run(ServiceAction.start.arguments(name: draft.name))
      let running = try await client.run(["status", "--json", "--", draft.name])
      let runningReport = try decoder.decode(StatusReport.self, from: Data(running.output.utf8))
      #expect(runningReport.status.processAlive)
      #expect(runningReport.status.identityMatches)
      _ = try await client.run(ServiceAction.stop.arguments(name: draft.name))
      let stopped = try await client.run(["status", "--json", "--", draft.name])
      let stoppedReport = try decoder.decode(StatusReport.self, from: Data(stopped.output.utf8))
      #expect(!stoppedReport.status.processAlive)
      #expect(!stoppedReport.status.identityMatches)
      #expect(stoppedReport.status.pid == nil)
    } catch {
      _ = try? await client.run(ServiceAction.stop.arguments(name: draft.name))
      throw error
    }
  }

  @Test(.enabled(if: ProcessInfo.processInfo.environment["PRESIDE_TEST_CLI"] != nil))
  func realCLIWholeSiteManagementUsesIsolatedHome() async throws {
    let executable = try #require(ProcessInfo.processInfo.environment["PRESIDE_TEST_CLI"])
    let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: home) }
    var environment = ProcessInfo.processInfo.environment
    environment["HOME"] = home.path
    let client = CLIClient(
      executableURL: URL(fileURLWithPath: executable), environment: environment, timeout: 15)
    var draft = ServiceDraft(
      name: "management", dir: home.path, command: "exec /bin/sleep 60", port: "54322",
      readiness: "process", environment: "PRESIDE_TEST=one")
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    _ = try await client.run(draft.arguments)
    do {
      _ = try await client.run(ServiceAction.start.arguments(name: draft.name))
      let before = try await client.run(["status", "--json", "--", draft.name])
      let beforeReport = try decoder.decode(StatusReport.self, from: Data(before.output.utf8))
      draft.environment = "PRESIDE_TEST=two"
      _ = try await client.run(draft.updateArguments(name: draft.name))
      let changed = try await client.run(["status", "--json", "--", draft.name])
      let changedReport = try decoder.decode(StatusReport.self, from: Data(changed.output.utf8))
      #expect(changedReport.status.stateMatchesConfig == false)
      _ = try await client.run(ServiceAction.restart.arguments(name: draft.name))
      let restarted = try await client.run(["status", "--json", "--", draft.name])
      let restartedReport = try decoder.decode(StatusReport.self, from: Data(restarted.output.utf8))
      #expect(restartedReport.status.pid != beforeReport.status.pid)
      #expect(restartedReport.status.stateMatchesConfig == true)
      _ = try await client.run(ServiceAction.stop.arguments(name: draft.name))
      _ = try await client.run(["rename", "--", draft.name, "renamed"])
      _ = try await client.run(["remove", "--", "renamed"])
      let remaining = try await client.run(["list", "--json"])
      #expect(
        try decoder.decode([String: ServiceConfig].self, from: Data(remaining.output.utf8)).isEmpty)
    } catch {
      _ = try? await client.run(["stop", "--", draft.name])
      _ = try? await client.run(["stop", "--", "renamed"])
      throw error
    }
  }

}

private struct ScriptFixture {
  let directory: URL
  let executable: URL

  init(_ body: String) throws {
    directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    executable = directory.appendingPathComponent("pp-fixture")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try Data("#!/bin/sh\n\(body)\n".utf8).write(to: executable)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: executable.path)
  }

  func remove() {
    try? FileManager.default.removeItem(at: directory)
  }
}
