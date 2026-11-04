import AppKit
import Foundation
import Observation
import UniformTypeIdentifiers

@MainActor
@Observable
final class ServiceStore {
  var services: [ServiceEntry] = []
  var selection: String?
  var showingAddSite = false
  var editingSite: ServiceEntry?
  var isRefreshing = false
  var isBusy = false
  var activeActionName: String?
  var activeAction: ServiceAction?
  var logSiteName: String?
  var logOutput = ""
  var logError: String?
  var isLoadingLogs = false
  private var loadingLogSiteName: String?
  private var logRequestID: UUID?
  private var actionError: String?
  private var refreshError: String?
  var error: String? {
    get { actionError ?? refreshError }
    set {
      actionError = newValue
      if newValue == nil { refreshError = nil }
    }
  }
  private(set) var notice: String?
  private var noticeSiteName: String?
  var doctorOutput: String?
  var executablePath: String {
    didSet { defaults.set(executablePath, forKey: "executablePath") }
  }
  var searchPath: String {
    didSet { defaults.set(searchPath, forKey: "searchPath") }
  }
  private let defaults: UserDefaults

  init(defaults: UserDefaults = .standard) {
    self.defaults = defaults
    executablePath = defaults.string(forKey: "executablePath") ?? ""
    searchPath = defaults.string(forKey: "searchPath") ?? Self.defaultSearchPath
  }

  static var defaultSearchPath: String {
    let home = FileManager.default.homeDirectoryForCurrentUser.path
    return [
      "\(home)/.cargo/bin", "\(home)/.local/bin", "/opt/homebrew/bin", "/usr/local/bin",
      ProcessInfo.processInfo.environment["PATH"] ?? "/usr/bin:/bin:/usr/sbin:/sbin",
    ]
    .joined(separator: ":")
  }

  private var client: CLIClient {
    var environment = ProcessInfo.processInfo.environment
    let resolvedSearchPath = SetupAssistance.normalizedSearchPath(searchPath)
    environment["PATH"] = resolvedSearchPath
    let executable: URL
    if !executablePath.isEmpty {
      executable = URL(fileURLWithPath: (executablePath as NSString).expandingTildeInPath)
    } else if let bundled = Bundle.main.resourceURL?.appendingPathComponent("pp"),
      FileManager.default.isExecutableFile(atPath: bundled.path)
    {
      executable = bundled
    } else {
      let paths = resolvedSearchPath.split(separator: ":").map {
        URL(fileURLWithPath: String($0)).appendingPathComponent("pp")
      }
      executable =
        paths.first { FileManager.default.isExecutableFile(atPath: $0.path) }
        ?? URL(fileURLWithPath: "/opt/homebrew/bin/pp")
    }
    return CLIClient(executableURL: executable, environment: environment)
  }

  func refresh() async {
    guard !isRefreshing, !isBusy else { return }
    isRefreshing = true
    defer { isRefreshing = false }
    await loadServices()
  }

  private func loadServices() async {
    let client = client
    do {
      let result = try await client.run(["list", "--json"])
      let decoder = JSONDecoder()
      decoder.keyDecodingStrategy = .convertFromSnakeCase
      let configurations = try decoder.decode(
        [String: ServiceConfig].self, from: Data(result.output.utf8))
      var entries: [ServiceEntry] = []
      for name in configurations.keys.sorted() {
        guard let config = configurations[name] else { continue }
        var entry = ServiceEntry(name: name, config: config)
        do {
          let output = try await client.run(["status", "--json", "--", name])
          entry.report = try decoder.decode(StatusReport.self, from: Data(output.output.utf8))
        } catch {
          entry.error = error.localizedDescription
        }
        entries.append(entry)
      }
      services = entries
      if !entries.contains(where: { $0.name == selection }) { selection = entries.first?.name }
      refreshError = nil
    } catch {
      refreshError = error.localizedDescription
      services = services.map {
        var entry = $0
        entry.report = nil
        entry.error = "Status unavailable. Refresh before controlling this service."
        return entry
      }
    }
  }

  private func beginOperation() async -> Bool {
    guard !isBusy else { return false }
    isBusy = true
    while isRefreshing {
      do { try await Task.sleep(for: .milliseconds(25)) } catch {
        isBusy = false
        return false
      }
    }
    return true
  }

  func notice(for name: String) -> String? {
    noticeSiteName == name ? notice : nil
  }

  func perform(_ action: ServiceAction, name: String) async {
    guard await beginOperation() else { return }
    activeAction = action
    activeActionName = name
    defer {
      activeAction = nil
      activeActionName = nil
    }
    error = nil
    notice = nil
    noticeSiteName = nil
    defer { isBusy = false }
    do {
      let result = try await client.run(action.arguments(name: name))
      noticeSiteName = name
      notice = [result.output, result.diagnostic].filter { !$0.isEmpty }.joined(separator: "\n")
      await loadServices()
    } catch {
      let failure = error.localizedDescription
      await loadServices()
      self.error = failure
    }
  }

  func add(_ draft: ServiceDraft) async -> Bool {
    await save(draft, replacing: nil)
  }

  func update(name: String, draft: ServiceDraft) async -> Bool {
    await save(draft, replacing: name)
  }

  private func save(_ draft: ServiceDraft, replacing name: String?) async -> Bool {
    guard !isBusy else { return false }
    if let validation = draft.validationError {
      error = validation
      return false
    }
    do {
      let arguments = try name.map { try draft.updateArguments(name: $0) } ?? draft.arguments
      return await mutate(arguments, targetName: name ?? draft.name)
    } catch {
      self.error = error.localizedDescription
      return false
    }
  }

  func rename(name: String, to newName: String) async -> Bool {
    await mutate(["rename", "--", name, newName], targetName: newName)
  }

  func remove(name: String) async -> Bool {
    await mutate(["remove", "--", name], targetName: name, selectTarget: false)
  }

  private func mutate(
    _ arguments: [String], targetName: String, selectTarget: Bool = true
  ) async -> Bool {
    guard await beginOperation() else { return false }
    error = nil
    notice = nil
    noticeSiteName = nil
    defer { isBusy = false }
    do {
      let result = try await client.run(arguments)
      noticeSiteName = targetName
      notice = result.output
      await loadServices()
      if selectTarget, services.contains(where: { $0.name == targetName }) {
        selection = targetName
      }
      return true
    } catch {
      self.error = error.localizedDescription
      return false
    }
  }

  func diagnose(name: String) async {
    guard await beginOperation() else { return }
    error = nil
    defer { isBusy = false }
    do {
      let result = try await client.run(
        ["doctor", "--json", "--", name], acceptedExitCodes: [0, 1])
      let report = try JSONDecoder().decode(DoctorReport.self, from: Data(result.output.utf8))
      doctorOutput = report.services.flatMap { service in
        [service.name] + service.checks.map { "\($0.ok ? "✓" : "!") \($0.name): \($0.message)" }
      }.joined(separator: "\n")
    } catch { self.error = error.localizedDescription }
  }

  func refreshLogs(name: String) async {
    guard !isLoadingLogs || loadingLogSiteName != name else { return }
    let requestID = UUID()
    logRequestID = requestID
    loadingLogSiteName = name
    isLoadingLogs = true
    defer {
      if logRequestID == requestID {
        isLoadingLogs = false
        loadingLogSiteName = nil
      }
    }
    do {
      let result = try await client.run(["logs", "--json", "--lines", "500", "--", name])
      let report = try JSONDecoder().decode(LogReport.self, from: Data(result.output.utf8))
      guard logSiteName == name, logRequestID == requestID else { return }
      logOutput = report.lines.joined(separator: "\n")
      logError = nil
    } catch {
      guard logSiteName == name, logRequestID == requestID else { return }
      logError = error.localizedDescription
    }
  }

  func copyLogs() {
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(logOutput, forType: .string)
  }

  func exportLogs(name: String) async {
    let snapshot = logOutput
    let panel = NSSavePanel()
    panel.nameFieldStringValue = "\(name)-logs.txt"
    panel.allowedContentTypes = [.plainText]
    guard await panel.begin() == .OK, let url = panel.url else { return }
    do { try snapshot.write(to: url, atomically: true, encoding: .utf8) } catch {
      logError = error.localizedDescription
    }
  }

  func revealConfig() async {
    do {
      let result = try await client.run(["path"])
      let url = URL(fileURLWithPath: result.output)
      guard FileManager.default.fileExists(atPath: url.path) else {
        throw CLIError(message: "Add your first service to create a configuration file.")
      }
      NSWorkspace.shared.activateFileViewerSelecting([url])
    } catch { self.error = error.localizedDescription }
  }
}

private struct DoctorReport: Decodable {
  var services: [DoctorService]
}

private struct DoctorService: Decodable {
  var name: String
  var checks: [DoctorCheck]
}

private struct DoctorCheck: Decodable {
  var name: String
  var ok: Bool
  var message: String
}

private struct LogReport: Decodable {
  var path: String
  var lines: [String]
}
