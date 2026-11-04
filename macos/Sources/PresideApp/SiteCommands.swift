import AppKit
import SwiftUI

struct SiteCommands: Commands {
  let store: ServiceStore
  @Environment(\.openWindow) private var openWindow

  var body: some Commands {
    CommandGroup(replacing: .newItem) {
      Button("Add Site…") {
        store.doctorOutput = nil
        store.showingAddSite = true
        openWindow(id: "services")
        NSApp.activate(ignoringOtherApps: true)
      }
      .keyboardShortcut("n")
      .disabled(store.isBusy)
    }
  }
}
