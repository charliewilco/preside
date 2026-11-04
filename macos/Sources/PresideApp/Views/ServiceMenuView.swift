import AppKit
import SwiftUI

struct ServiceMenuView: View {
  @Bindable var store: ServiceStore
  @Environment(\.openWindow) private var openWindow

  var body: some View {
    Button("Open Preside") {
      openWindow(id: "services")
      NSApp.activate(ignoringOtherApps: true)
    }
    Button("Add Site…", systemImage: "plus") {
      store.editingSite = nil
      store.doctorOutput = nil
      store.showingAddSite = true
      openWindow(id: "services")
      NSApp.activate(ignoringOtherApps: true)
    }
    .disabled(store.isBusy)
    Divider()
    if store.services.isEmpty {
      Text("No sites configured")
    }
    ForEach(store.services) { service in
      Menu("\(service.name) — \(service.stateLabel)") {
        Text(service.config.kind == "static" ? "Static folder" : "Server command")
        Button("Start") { Task { await store.perform(.start, name: service.name) } }
          .disabled(store.isBusy || !service.canStart)
        Button("Stop") { Task { await store.perform(.stop, name: service.name) } }
          .disabled(store.isBusy || !service.canStop)
        Button("Restart") { Task { await store.perform(.restart, name: service.name) } }
          .disabled(store.isBusy || !service.canStop)
        Button("Edit Site…") {
          store.showingAddSite = false
          store.doctorOutput = nil
          store.error = nil
          store.editingSite = service
          openWindow(id: "services")
          NSApp.activate(ignoringOtherApps: true)
        }
        .disabled(
          store.isBusy || service.error != nil || service.report == nil
            || (service.report?.status.processAlive == true
              && service.report?.status.identityMatches != true)
        )
        Button("Open in Browser") { Task { await store.perform(.open, name: service.name) } }
          .disabled(store.isBusy || service.report?.status.url == nil)
        Button("Show Details") {
          store.selection = service.name
          openWindow(id: "services")
          NSApp.activate(ignoringOtherApps: true)
        }
      }
    }
    Divider()
    if store.error != nil {
      Text("An action failed. Open Preside for details.")
    }
    Button("Refresh") { Task { await store.refresh() } }
      .disabled(store.isRefreshing || store.isBusy)
    SettingsLink()
    Divider()
    Button("Quit Preside") { NSApp.terminate(nil) }.keyboardShortcut("q")
  }
}

#Preview {
  ServiceMenuView(store: ServiceStore())
}
