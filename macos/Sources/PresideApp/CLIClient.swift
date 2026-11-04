import Darwin
import Foundation

struct CommandResult: Sendable {
  var output: String
  var diagnostic: String
  var exitCode: Int32
}

struct CLIError: LocalizedError {
  var message: String
  var errorDescription: String? { message }
}

struct CLIClient: Sendable {
  var executableURL: URL
  var environment: [String: String] = ProcessInfo.processInfo.environment
  var timeout: TimeInterval = 30

  func run(_ arguments: [String], acceptedExitCodes: Set<Int32> = [0]) async throws -> CommandResult
  {
    let result = try await withCheckedThrowingContinuation {
      (continuation: CheckedContinuation<CommandResult, Error>) in
      DispatchQueue.global(qos: .utility).async {
        do { continuation.resume(returning: try execute(arguments)) } catch {
          continuation.resume(throwing: error)
        }
      }
    }
    guard acceptedExitCodes.contains(result.exitCode) else {
      let detail = result.diagnostic.isEmpty ? result.output : result.diagnostic
      throw CLIError(
        message: detail.isEmpty ? "Preside exited with code \(result.exitCode)." : detail)
    }
    return result
  }

  private func execute(_ arguments: [String]) throws -> CommandResult {
    guard FileManager.default.isExecutableFile(atPath: executableURL.path) else {
      throw CLIError(
        message:
          "Cannot run pp at \(executableURL.path). Choose a Preside executable in Settings or rebuild the app bundle."
      )
    }
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(
      at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
    defer { try? FileManager.default.removeItem(at: directory) }
    let outputURL = directory.appendingPathComponent("stdout")
    let errorURL = directory.appendingPathComponent("stderr")
    FileManager.default.createFile(
      atPath: outputURL.path, contents: nil, attributes: [.posixPermissions: 0o600])
    FileManager.default.createFile(
      atPath: errorURL.path, contents: nil, attributes: [.posixPermissions: 0o600])
    let output = try FileHandle(forWritingTo: outputURL)
    let diagnostic = try FileHandle(forWritingTo: errorURL)
    defer {
      try? output.close()
      try? diagnostic.close()
    }
    let process = Process()
    process.executableURL = executableURL
    process.arguments = arguments
    process.environment = environment
    process.standardInput = FileHandle.nullDevice
    process.standardOutput = output
    process.standardError = diagnostic
    try process.run()
    let deadline = Date().addingTimeInterval(timeout)
    while process.isRunning && Date() < deadline { Thread.sleep(forTimeInterval: 0.025) }
    if process.isRunning {
      process.terminate()
      let grace = Date().addingTimeInterval(1)
      while process.isRunning && Date() < grace { Thread.sleep(forTimeInterval: 0.025) }
      if process.isRunning { kill(process.processIdentifier, SIGKILL) }
      process.waitUntilExit()
      throw CLIError(
        message:
          "pp timed out. A service may still be running; refresh its status before trying again.")
    }
    process.waitUntilExit()
    return try CommandResult(
      output: readOutput(outputURL), diagnostic: readOutput(errorURL),
      exitCode: process.terminationStatus
    )
  }

  private func readOutput(_ url: URL) throws -> String {
    let handle = try FileHandle(forReadingFrom: url)
    defer { try? handle.close() }
    let data = try handle.read(upToCount: 4 * 1024 * 1024 + 1) ?? Data()
    guard data.count <= 4 * 1024 * 1024 else {
      throw CLIError(
        message: "pp returned more than 4 MiB of output. Inspect this service in the terminal.")
    }
    return String(decoding: data, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
  }
}
