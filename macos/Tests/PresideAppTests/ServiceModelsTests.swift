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
        "--readiness", "port", "--url-mode", "direct", "--env", "TOKEN=a=b;$(touch unwanted)",
        "--env", "EMPTY=",
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
      draft.arguments == [
        "add", "--dir", draft.dir, "--port", "8123", "--static", "--url-mode", "direct", "--",
        "app",
      ])
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

  @Test func editPreservesMultilineEnvironmentAndAdvancedStaticSettings() throws {
    let entry = ServiceEntry(
      name: "docs",
      config: ServiceConfig(
        dir: "/tmp/docs", port: 8123, readiness: "http", healthPath: "/health",
        env: ["CERT": "line1\nline2", "EMPTY": ""], kind: "static"
      ))
    var draft = ServiceDraft(entry: entry)
    draft.port = "8124"
    #expect(draft.validationError == nil)
    let arguments = try draft.updateArguments(name: entry.name)
    #expect(arguments.first == "update")
    #expect(arguments.last == "docs")
    #expect(arguments.contains("--static"))
    #expect(arguments.contains("CERT=line1\nline2"))
    #expect(arguments.contains("EMPTY="))
    #expect(arguments.contains("http"))
    #expect(arguments.contains("/health"))
    #expect(arguments.contains("8124"))
  }

  @Test func commandEditPreservesUntouchedMultilineValues() throws {
    let entry = ServiceEntry(
      name: "worker",
      config: ServiceConfig(
        dir: "/tmp/app", command: "run-server", port: 8123, readiness: "port",
        healthPath: "/health", env: ["KEY": "a\nb"]
      ))
    var draft = ServiceDraft(entry: entry)
    draft.command = "run-another-server"
    #expect(draft.validationError == nil)
    #expect(try draft.updateArguments(name: "worker").contains("KEY=a\nb"))
    #expect(try draft.updateArguments(name: "worker").contains("/health"))
    draft.environment = "KEY=replaced"
    #expect(draft.arguments.contains("KEY=replaced"))
    #expect(!draft.arguments.contains("KEY=a\nb"))
    #expect(ServiceAction.restart.arguments(name: "worker") == ["restart", "--", "worker"])
  }

  @Test func structuredEnvironmentEditsPreserveMultilineValues() throws {
    var draft = ServiceDraft(
      entry: ServiceEntry(
        name: "app",
        config: ServiceConfig(
          dir: "/tmp/app", command: "run", port: 8123,
          env: ["CERT": "first\nSECOND=part", "OTHER": "old"])))
    draft.prepareEnvironmentEditor()
    let index = try #require(draft.environmentVariables?.firstIndex { $0.key == "OTHER" })
    draft.environmentVariables?[index].value = "changed"
    #expect(draft.validationError == nil)
    #expect(draft.arguments.contains("CERT=first\nSECOND=part"))
    #expect(draft.arguments.contains("OTHER=changed"))
    #expect(!draft.arguments.contains("SECOND=part"))
    let update = try draft.updateArguments(name: "app")
    let expectedIndex = try #require(update.firstIndex(of: "--expected-config"))
    let expected = try JSONDecoder().decode(
      ServiceConfig.self, from: Data(update[expectedIndex + 1].utf8))
    #expect(expected.env?["OTHER"] == "old")
  }

  @Test(arguments: [
    ("starting", "Starting"), ("ready", "Ready"), ("unhealthy", "Unhealthy"),
    ("failed", "Failed"), ("unknown", "Unknown process"), ("future-phase", "Running"),
  ])
  func runtimePhasesPreserveIndependentProcessControls(phase: String, label: String) {
    var service = entry(alive: true, identity: true)
    service.report?.status.phase = phase
    #expect(service.stateLabel == label)
    #expect(service.canStop)
    #expect(!service.canStart)
    service.report?.status.identityMatches = false
    #expect(service.stateLabel == "Unknown process")
    #expect(!service.canStop)
  }

  @Test func readinessDecodesAndLegacyStatusRemainsCompatible() throws {
    let json =
      #"{"process_alive":true,"identity_matches":true,"recent_log_lines":[],"log_path":"/tmp/app.log","phase":"unhealthy","readiness_passed":false,"readiness_message":"HTTP returned 503"}"#
    let status = try decoder().decode(RuntimeStatus.self, from: Data(json.utf8))
    #expect(status.phase == "unhealthy")
    #expect(status.readinessPassed == false)
    #expect(status.readinessMessage == "HTTP returned 503")
    #expect(entry(alive: true, identity: true).report?.status.phase == nil)
    #expect(entry(alive: true, identity: true).stateLabel == "Running")
  }

  @Test(arguments: ["command", "static"])
  func urlModeDefaultsAndLegacyEditsPreserveIntent(kind: String) throws {
    var fresh = validDraft()
    fresh.kind = kind
    #expect(fresh.urlMode == "direct")
    let index = try #require(fresh.arguments.firstIndex(of: "--url-mode"))
    #expect(fresh.arguments[index + 1] == "direct")
    let legacy = ServiceEntry(
      name: "legacy",
      config: ServiceConfig(dir: "/tmp/site", command: "run", port: 8123, kind: kind))
    var edited = ServiceDraft(entry: legacy)
    #expect(edited.urlMode == nil)
    #expect(!edited.arguments.contains("--url-mode"))
    let update = try edited.updateArguments(name: legacy.name)
    let expectedIndex = try #require(update.firstIndex(of: "--expected-config"))
    let snapshot =
      try JSONSerialization.jsonObject(with: Data(update[expectedIndex + 1].utf8)) as? [String: Any]
    #expect(snapshot?["url_mode"] == nil)
    edited.urlMode = "proxy"
    let changed = try edited.updateArguments(name: legacy.name)
    let modeIndex = try #require(changed.firstIndex(of: "--url-mode"))
    #expect(changed[modeIndex + 1] == "proxy")
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
