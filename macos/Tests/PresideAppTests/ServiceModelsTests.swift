import Foundation
import Testing

@testable import PresideApp

struct ServiceModelsTests {
  private func decoder() -> JSONDecoder {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return decoder
  }

  @Test func configurationDecodesSnakeCaseAndOmittedFields() throws {
    let json =
      #"{"complete":{"dir":"~/Code/app","command":"npm run dev","port":8123,"readiness":"http","health_path":"/healthz","env":{"TOKEN":"a=b"}},"incomplete":{"port":9000}}"#
    let configurations = try decoder().decode([String: ServiceConfig].self, from: Data(json.utf8))
    let complete = try #require(configurations["complete"])
    #expect(complete.healthPath == "/healthz")
    #expect(complete.env == ["TOKEN": "a=b"])
    #expect(complete.domain == nil)
    let incomplete = try #require(configurations["incomplete"])
    #expect(incomplete.port == 9000)
    #expect(incomplete.dir == nil)
    #expect(incomplete.command == nil)
    #expect(incomplete.env == nil)
  }

  @Test func statusDecodesActualCLIShape() throws {
    let json =
      #"{"name":"app","service":{"port":8123},"missing_fields":["dir","command"],"status":{"process_alive":false,"identity_matches":false,"listening":null,"state_matches_config":null,"pid":null,"url":null,"recent_log_lines":["ready"],"log_path":"/tmp/app.log","state":null,"pid_path":"/tmp/app.pid","pid_file_exists":false}}"#
    let report = try decoder().decode(StatusReport.self, from: Data(json.utf8))
    #expect(report.missingFields == ["dir", "command"])
    #expect(report.status.recentLogLines == ["ready"])
    #expect(report.status.logPath == "/tmp/app.log")
    #expect(report.status.pid == nil)
    #expect(report.status.stateMatchesConfig == nil)
  }

  @Test func unknownProcessCannotBeStartedOrStopped() {
    let entry = entry(alive: true, identity: false)
    #expect(entry.stateLabel == "Unknown process")
    #expect(!entry.canStart)
    #expect(!entry.canStop)
  }

  @Test func unavailableAndIncompleteServicesCannotBeControlled() {
    var unavailable = entry(alive: true, identity: true)
    unavailable.error = "Cannot refresh"
    #expect(!unavailable.canStart)
    #expect(!unavailable.canStop)
    #expect(unavailable.stateLabel == "Unavailable")
    unavailable.error = nil
    unavailable.report = nil
    #expect(!unavailable.canStart)
    #expect(!unavailable.canStop)
    let incomplete = entry(alive: false, identity: false, missing: ["command"])
    #expect(incomplete.stateLabel == "Incomplete")
    #expect(!incomplete.canStart)
  }

  @Test func knownRunningAndStoppedServicesExposeAppropriateActions() {
    let running = entry(alive: true, identity: true)
    #expect(running.canStop)
    #expect(!running.canStart)
    let stopped = entry(alive: false, identity: false)
    #expect(stopped.canStart)
    #expect(!stopped.canStop)
  }

  @Test func draftPreservesShellMetacharactersAsSingleArguments() {
    var draft = validDraft()
    draft.dir = "/tmp/project with spaces;$(touch unwanted)"
    draft.command = #"printf '%s' "$TOKEN" && npm run dev"#
    draft.environment = "TOKEN=a=b;$(touch unwanted)\nEMPTY="
    #expect(draft.validationError == nil)
    #expect(
      draft.arguments == [
        "add", "--dir", draft.dir, "--command", draft.command, "--port", "8123",
        "--readiness", "port", "--env", "TOKEN=a=b;$(touch unwanted)", "--env", "EMPTY=",
        "--", "app",
      ])
    #expect(ServiceAction.start.arguments(name: "app") == ["run", "--detach", "--", "app"])
  }

  @Test(arguments: ["0", "65536", "-1", "eight", ""])
  func draftRejectsInvalidPorts(_ port: String) {
    var draft = validDraft()
    draft.port = port
    #expect(draft.validationError != nil)
  }

  @Test(arguments: ["", "-app", "_app", "app name", "../app"])
  func draftRejectsInvalidServiceNames(_ name: String) {
    var draft = validDraft()
    draft.name = name
    #expect(draft.validationError != nil)
  }

  @Test func draftAllowsDotsInServiceNames() {
    var draft = validDraft()
    draft.name = "app.preview"
    #expect(draft.validationError == nil)
  }

  @Test(arguments: ["MISSING_EQUALS", "1TOKEN=value", "BAD-KEY=value", "=value"])
  func draftRejectsInvalidEnvironment(_ environment: String) {
    var draft = validDraft()
    draft.environment = environment
    #expect(draft.validationError != nil)
  }

  @Test(arguments: ["health", "/has space", ""])
  func draftRejectsInvalidHTTPHealthPaths(_ healthPath: String) {
    var draft = validDraft()
    draft.readiness = "http"
    draft.healthPath = healthPath
    #expect(draft.validationError != nil)
  }

  @Test func staticDraftNeedsNoCommandAndDoesNotLeakHiddenCommandOptions() {
    var draft = validDraft()
    draft.kind = "static"
    draft.command = ""
    draft.readiness = "http"
    draft.healthPath = "invalid stale path"
    draft.environment = "invalid stale environment"
    draft.dir = "/tmp/site with spaces;$(touch unwanted)"
    #expect(draft.validationError == nil)
    #expect(
      draft.arguments == ["add", "--dir", draft.dir, "--port", "8123", "--static", "--", "app"])
    draft.kind = "command"
    #expect(draft.validationError != nil)
  }

  @Test func staticConfigurationDecodesWithoutCommand() throws {
    let json = #"{"dir":"/tmp/site","port":8123,"kind":"static"}"#
    let config = try decoder().decode(ServiceConfig.self, from: Data(json.utf8))
    #expect(config.kind == "static")
    #expect(config.command == nil)
    #expect(config.dir == "/tmp/site")
  }

  private func validDraft() -> ServiceDraft {
    ServiceDraft(name: "app", dir: "/tmp/project", command: "npm run dev", port: "8123")
  }

  private func entry(alive: Bool, identity: Bool, missing: [String] = []) -> ServiceEntry {
    ServiceEntry(
      name: "app", config: ServiceConfig(),
      report: StatusReport(
        status: RuntimeStatus(
          processAlive: alive, identityMatches: identity, recentLogLines: [],
          logPath: "/tmp/app.log"
        ), missingFields: missing
      )
    )
  }
}
