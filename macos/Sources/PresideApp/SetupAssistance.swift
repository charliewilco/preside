import Darwin
import Foundation

struct SetupAssistance {
  static func suggestPort() throws -> Int {
    let descriptor = socket(AF_INET, SOCK_STREAM, 0)
    guard descriptor >= 0 else { throw CLIError(message: "Could not check available ports.") }
    defer { close(descriptor) }
    var address = sockaddr_in()
    address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
    address.sin_family = sa_family_t(AF_INET)
    address.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))
    address.sin_port = 0
    let bound = withUnsafePointer(to: &address) { pointer in
      pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        bind(descriptor, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
      }
    }
    guard bound == 0 else { throw CLIError(message: "Could not find an available loopback port.") }
    var length = socklen_t(MemoryLayout<sockaddr_in>.size)
    let result = withUnsafeMutablePointer(to: &address) { pointer in
      pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        getsockname(descriptor, $0, &length)
      }
    }
    guard result == 0 else { throw CLIError(message: "Could not read the suggested port.") }
    return Int(UInt16(bigEndian: address.sin_port))
  }

  static func normalizedSearchPath(_ searchPath: String) -> String {
    searchPath.split(separator: ":", omittingEmptySubsequences: false)
      .map { (String($0) as NSString).expandingTildeInPath }
      .joined(separator: ":")
  }

  static func executable(tool: String, searchPath: String) -> String? {
    let name = tool.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty else { return nil }
    let candidates: [String]
    if name.hasPrefix("/") || name.hasPrefix("~/") {
      candidates = [(name as NSString).expandingTildeInPath]
    } else if name.contains("/") {
      return nil
    } else {
      candidates = searchPath.split(separator: ":").compactMap { directory in
        let path = (String(directory) as NSString).expandingTildeInPath
        guard path.hasPrefix("/") else { return nil }
        return URL(fileURLWithPath: path).appendingPathComponent(name).path
      }
    }
    return candidates.first { path in
      var isDirectory: ObjCBool = false
      return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
        && !isDirectory.boolValue && FileManager.default.isExecutableFile(atPath: path)
    }
  }

  static func prepending(directory: String, to searchPath: String) -> String {
    ([directory] + searchPath.split(separator: ":").map(String.init).filter { $0 != directory })
      .joined(separator: ":")
  }
}
