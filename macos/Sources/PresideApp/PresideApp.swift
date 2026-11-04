import AppKit
import SwiftUI

@main
struct PresideApp: App {
  @NSApplicationDelegateAdaptor(PresideAppDelegate.self) private var delegate

  private var store: ServiceStore { delegate.store }

  var body: some Scene {
    Window("Preside", id: "services") {
      ContentView(store: store)
        .frame(minWidth: 800, minHeight: 520)
    }
    .defaultSize(width: 1040, height: 680)
    .commands {
      SiteCommands(store: store)
      CommandGroup(after: .newItem) {
        Button("Refresh Sites") { Task { await store.refresh() } }
          .keyboardShortcut("r")
          .disabled(store.isBusy || store.isRefreshing)
      }
    }

    MenuBarExtra {
      ServiceMenuView(store: store)
    } label: {
      Label("Preside", systemImage: "terminal")

    }

    Settings {
      SettingsView(store: store)
    }
  }
}
