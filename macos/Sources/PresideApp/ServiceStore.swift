import AppKit
import Foundation
import Observation

@MainActor
@Observable
final class ServiceStore {
  var services: [ServiceEntry] = []
  var selection: String?
  var showingAddSite = false
  var isRefreshing = false
  var isBusy = false
  private var actionError: String?
  private var refreshError: String?
  var error: String? {
    get { actionError ?? refreshError }
    set {
      actionError = newValue
      if newValue == nil { refreshError = nil }
    }
  }
  var notice: String?
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
    environment["PATH"] = searchPath
    let executable: URL
    if !executablePath.isEmpty {
      executable = URL(fileURLWithPath: (executablePath as NSString).expandingTildeInPath)
    } else if let bundled = Bundle.main.resourceURL?.appendingPathComponent("pp"),
      FileManager.default.isExecutableFile(atPath: bundled.path)
    {
      executable = bundled
    } else {
      let paths = searchPath.split(separator: ":").map {
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

  func perform(_ action: ServiceAction, name: String) async {
    guard await beginOperation() else { return }
    error = nil
    notice = nil
    defer { isBusy = false }
    do {
      let result = try await client.run(action.arguments(name: name))
      notice = [result.output, result.diagnostic].filter { !$0.isEmpty }.joined(separator: "\n")
      await loadServices()
    } catch {
      let failure = error.localizedDescription
      await loadServices()
      self.error = failure
    }
  }

  func add(_ draft: ServiceDraft) async -> Bool {
    guard !isBusy else { return false }
    if let validation = draft.validationError {
      error = validation
      return false
    }
    guard await beginOperation() else { return false }
    error = nil
    notice = nil
    defer { isBusy = false }
    do {
      let result = try await client.run(draft.arguments)
      notice = result.output
      await loadServices()
      selection = draft.name
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
