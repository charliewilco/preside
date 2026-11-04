import AppKit

@MainActor
final class PresideAppDelegate: NSObject, NSApplicationDelegate {
  let store = ServiceStore()
  private var pollingTask: Task<Void, Never>?

  func applicationDidFinishLaunching(_ notification: Notification) {
    pollingTask = Task {
      while !Task.isCancelled {
        await store.refresh()
        do { try await Task.sleep(for: .seconds(3)) } catch { return }
      }
    }
  }

  func applicationWillTerminate(_ notification: Notification) {
    pollingTask?.cancel()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    false
  }
}
