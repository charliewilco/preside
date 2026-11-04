import Darwin
import Foundation
import Testing

@testable import PresideApp

struct SetupAssistanceTests {
  @Test func executableLookupSkipsDirectoriesAndNonExecutableFiles() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let first = root.appendingPathComponent("first")
    let second = root.appendingPathComponent("second with spaces")
    try FileManager.default.createDirectory(at: first, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: second, withIntermediateDirectories: true)
    let blocked = first.appendingPathComponent("tool")
    let valid = second.appendingPathComponent("tool")
    try Data("#!/bin/sh\nexit 0\n".utf8).write(to: blocked)
    try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: blocked.path)
    try Data("#!/bin/sh\nexit 0\n".utf8).write(to: valid)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: valid.path)
    let searchPath = "\(first.path):\(second.path)"
    #expect(SetupAssistance.executable(tool: " tool\n", searchPath: searchPath) == valid.path)
    #expect(SetupAssistance.executable(tool: valid.path, searchPath: "") == valid.path)
    #expect(SetupAssistance.executable(tool: first.path, searchPath: searchPath) == nil)
    #expect(SetupAssistance.executable(tool: "./tool", searchPath: searchPath) == nil)
    #expect(SetupAssistance.executable(tool: "tool", searchPath: ":relative/bin:") == nil)
    #expect(SetupAssistance.executable(tool: " ", searchPath: searchPath) == nil)
    #expect(SetupAssistance.executable(tool: "missing-tool", searchPath: searchPath) == nil)
  }

  @Test func pathNormalizationExpandsHomeAndPreservesOrdering() {
    let homeBin = ("~/bin" as NSString).expandingTildeInPath
    #expect(
      SetupAssistance.normalizedSearchPath("~/bin::/usr/bin:relative")
        == "\(homeBin)::/usr/bin:relative")
    #expect(
      SetupAssistance.prepending(
        directory: "/custom bin", to: "/usr/bin:/custom bin:/bin:/custom bin")
        == "/custom bin:/usr/bin:/bin")
  }

  @Test func suggestedPortIsUsableOnLoopbackAndReleased() throws {
    let port = try SetupAssistance.suggestPort()
    #expect((1...65535).contains(port))
    let descriptor = socket(AF_INET, SOCK_STREAM, 0)
    #expect(descriptor >= 0)
    guard descriptor >= 0 else { return }
    defer { close(descriptor) }
    var address = sockaddr_in()
    address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
    address.sin_family = sa_family_t(AF_INET)
    address.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))
    address.sin_port = UInt16(port).bigEndian
    let result = withUnsafePointer(to: &address) { pointer in
      pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        bind(descriptor, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
      }
    }
    #expect(result == 0)
  }
}
