import Foundation

struct ServiceConfig: Codable, Sendable {
  var dir: String?
  var command: String?
  var port: Int?
  var domain: String?
  var readiness: String?
  var healthPath: String?
  var env: [String: String]?
  var kind: String?
}

struct StatusReport: Decodable, Sendable {
  var status: RuntimeStatus
  var missingFields: [String]
}

struct RuntimeStatus: Decodable, Sendable {
  var processAlive: Bool
  var identityMatches: Bool
  var listening: Bool?
  var stateMatchesConfig: Bool?
  var pid: Int?
  var url: String?
  var recentLogLines: [String]
  var logPath: String
}

struct ServiceEntry: Identifiable, Sendable {
  var id: String { name }
  var name: String
  var config: ServiceConfig
  var report: StatusReport?
  var error: String?

  var stateLabel: String {
    guard error == nil, let report else { return "Unavailable" }
    if report.status.processAlive && !report.status.identityMatches { return "Unknown process" }
    if report.status.identityMatches { return "Running" }
    if !report.missingFields.isEmpty { return "Incomplete" }
    return "Stopped"
  }

  var canStart: Bool {
    guard error == nil, let report else { return false }
    return !report.status.processAlive && report.missingFields.isEmpty
  }

  var canStop: Bool {
    error == nil && report?.status.identityMatches == true
  }
}

enum ServiceAction: Sendable {
  case start, stop, restart, open

  func arguments(name: String) -> [String] {
    switch self {
    case .start: ["run", "--detach", "--", name]
    case .stop: ["stop", "--", name]
    case .restart: ["restart", "--", name]
    case .open: ["open", "--", name]
    }
  }
}

struct EnvironmentVariable: Identifiable {
  var id = UUID()
  var key = ""
  var value = ""
}

struct ServiceDraft {
  var name = ""
  var dir = ""
  var command = ""
  var port = ""
  var domain = ""
  var readiness = "port"
  var healthPath = "/"
  var environment = ""
  var kind = "command"
  var originalConfig: ServiceConfig?
  var originalEnvironment: String?
  var environmentVariables: [EnvironmentVariable]?

  var validationError: String? {
    if name.isEmpty
      || name.range(of: "^[A-Za-z0-9][A-Za-z0-9_.-]*$", options: .regularExpression) == nil
    {
      return
        "Start the name with a letter or number; use letters, numbers, dots, underscores, or hyphens."
    }
    if dir.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Choose a project directory."
    }
    if kind == "command" && command.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Enter a command to run."
    }
    guard let portNumber = Int(port), (1...65535).contains(portNumber) else {
      return "Enter a port between 1 and 65535."
    }
    if kind == "static" { return nil }
    if readiness == "http"
      && (!healthPath.hasPrefix("/") || healthPath.contains(where: \.isWhitespace))
    {
      return "The health path must begin with / and contain no whitespace."
    }
    if let environmentVariables {
      var keys = Set<String>()
      for variable in environmentVariables {
        if variable.key.range(of: "^[A-Za-z_][A-Za-z0-9_]*$", options: .regularExpression) == nil {
          return
            "Environment names must start with a letter or underscore and contain only letters, numbers, or underscores."
        }
        if !keys.insert(variable.key).inserted {
          return "Each environment variable needs a unique name."
        }
        if variable.value.contains("\0") {
          return "Environment values cannot contain null characters."
        }
      }
      return nil
    }
    if environment == originalEnvironment { return nil }
    for line in environment.split(separator: "\n") {
      let pair = line.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
      if pair.count != 2
        || pair[0].range(of: "^[A-Za-z_][A-Za-z0-9_]*$", options: .regularExpression) == nil
      {
        return "Enter one environment variable per line as KEY=value."
      }
    }
    return nil
  }

  var arguments: [String] {
    if kind == "static" {
      var result = ["add", "--dir", dir, "--port", port, "--static"]
      if !domain.isEmpty { result += ["--domain", domain] }
      if originalConfig?.kind == "static" {
        if let readiness = originalConfig?.readiness { result += ["--readiness", readiness] }
        if let healthPath = originalConfig?.healthPath { result += ["--health-path", healthPath] }
        for key in (originalConfig?.env ?? [:]).keys.sorted() {
          result += ["--env", "\(key)=\(originalConfig?.env?[key] ?? "")"]
        }
      }
      return result + ["--", name]
    }
    var result = [
      "add", "--dir", dir, "--command", command, "--port", port, "--readiness", readiness,
    ]
    if !domain.isEmpty { result += ["--domain", domain] }
    if readiness == "http" {
      result += ["--health-path", healthPath]
    } else if let path = originalConfig?.healthPath {
      result += ["--health-path", path]
    }
    if let environmentVariables {
      for variable in environmentVariables {
        result += ["--env", "\(variable.key)=\(variable.value)"]
      }
    } else if environment == originalEnvironment, let existing = originalConfig?.env {
      for key in existing.keys.sorted() { result += ["--env", "\(key)=\(existing[key] ?? "")"] }
    } else {
      for line in environment.split(separator: "\n") { result += ["--env", String(line)] }
    }
    return result + ["--", name]
  }
}

extension ServiceDraft {
  mutating func prepareEnvironmentEditor() {
    guard environmentVariables == nil else { return }
    if let existing = originalConfig?.env {
      environmentVariables = existing.sorted { $0.key < $1.key }.map {
        EnvironmentVariable(key: $0.key, value: $0.value)
      }
    } else {
      environmentVariables = environment.split(separator: "\n").compactMap { line in
        let parts = line.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        return EnvironmentVariable(key: String(parts[0]), value: String(parts[1]))
      }
    }
  }

  init(entry: ServiceEntry) {
    self.init()
    name = entry.name
    dir = entry.config.dir ?? ""
    command = entry.config.command ?? ""
    port = entry.config.port.map(String.init) ?? ""
    domain = entry.config.domain ?? ""
    readiness = entry.config.readiness ?? "port"
    healthPath = entry.config.healthPath ?? "/"
    kind = entry.config.kind ?? "command"
    environment = (entry.config.env ?? [:]).sorted { $0.key < $1.key }
      .map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
    originalConfig = entry.config
    originalEnvironment = environment
  }

  func updateArguments(name: String) throws -> [String] {
    var result = arguments
    result[0] = "update"
    result[result.count - 1] = name
    if let originalConfig {
      let encoder = JSONEncoder()
      encoder.keyEncodingStrategy = .convertToSnakeCase
      let expected = try encoder.encode(originalConfig)
      result.insert(
        contentsOf: ["--expected-config", String(decoding: expected, as: UTF8.self)], at: 1)
    }
    return result
  }
}
