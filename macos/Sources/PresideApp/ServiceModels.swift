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
  case start, stop, open

  func arguments(name: String) -> [String] {
    switch self {
    case .start: ["run", "--detach", "--", name]
    case .stop: ["stop", "--", name]
    case .open: ["open", "--", name]
    }
  }
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
      return result + ["--", name]
    }
    var result = [
      "add", "--dir", dir, "--command", command, "--port", port, "--readiness", readiness,
    ]
    if !domain.isEmpty { result += ["--domain", domain] }
    if readiness == "http" { result += ["--health-path", healthPath] }
    for line in environment.split(separator: "\n") { result += ["--env", String(line)] }
    return result + ["--", name]
  }
}
