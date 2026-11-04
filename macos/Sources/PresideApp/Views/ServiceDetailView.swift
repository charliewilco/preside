import AppKit
import SwiftUI

struct ServiceDetailView: View {
  @Bindable var store: ServiceStore
  let service: ServiceEntry
  @State private var renamingSite: ServiceEntry?
  @State private var showingRemoveConfirmation = false
  @State private var removalSite: ServiceEntry?

  private var canManage: Bool {
    guard service.error == nil, let status = service.report?.status else { return false }
    return !status.processAlive || status.identityMatches
  }

  private var canRenameOrRemove: Bool {
    canManage && service.report?.status.processAlive == false
  }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 24) {
        HStack(alignment: .top) {
          VStack(alignment: .leading, spacing: 6) {
            Text(service.name).font(.largeTitle.bold())
            Label(service.stateLabel, systemImage: service.canStop ? "play.circle.fill" : "circle")
              .foregroundStyle(service.canStop ? Color.green : Color.secondary)
          }
          Spacer()
          Menu("Manage Site", systemImage: "ellipsis.circle") {
            Button("Edit Site…", systemImage: "pencil") {
              store.error = nil
              store.editingSite = service
            }
            .disabled(store.isBusy || !canManage)
            Button("Rename…") {
              store.error = nil
              renamingSite = service
            }
            .disabled(store.isBusy || !canRenameOrRemove)
            Divider()
            Button("Remove Site…", role: .destructive) {
              removalSite = service
              showingRemoveConfirmation = true
            }
            .disabled(store.isBusy || !canRenameOrRemove)
            if !canManage {
              Text("Resolve this site’s status before changing it.")
            } else if !canRenameOrRemove {
              Text("Stop the site before renaming or removing it.")
            }
          }
          .fixedSize()
        }

        HStack {
          Button(
            service.canStop ? "Stop" : "Start",
            systemImage: service.canStop ? "stop.fill" : "play.fill"
          ) {
            Task { await store.perform(service.canStop ? .stop : .start, name: service.name) }
          }
          .buttonStyle(.borderedProminent)
          .disabled(store.isBusy || !(service.canStart || service.canStop))
          Button("Restart", systemImage: "arrow.clockwise") {
            Task { await store.perform(.restart, name: service.name) }
          }
          .disabled(store.isBusy || !service.canStop)
          Button("Open", systemImage: "arrow.up.right.square") {
            Task { await store.perform(.open, name: service.name) }
          }
          .disabled(store.isBusy || service.report?.status.url == nil)
          Spacer()
          if store.isBusy {
            ProgressView().controlSize(.small).accessibilityLabel("Running site action")
          }
        }

        if let error = service.error {
          Label(error, systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
            .textSelection(.enabled)
        }
        if let notice = store.notice {
          Text(notice)
            .font(.callout)
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
        }
        if let report = service.report {
          if !report.missingFields.isEmpty {
            Label(
              "Missing configuration: \(report.missingFields.joined(separator: ", "))",
              systemImage: "exclamationmark.triangle"
            )
            .foregroundStyle(.orange)
          }
          if report.status.processAlive && report.status.stateMatchesConfig == false {
            Label(
              "The running site uses older configuration. Restart it to apply your changes.",
              systemImage: "exclamationmark.triangle"
            )
            .foregroundStyle(.orange)
          }
        }

        GroupBox {
          Grid(alignment: .leading, horizontalSpacing: 24, verticalSpacing: 12) {
            GridRow {
              Text("Directory").foregroundStyle(.secondary)
              Text(service.config.dir ?? "Not set")
            }
            GridRow {
              Text("Type").foregroundStyle(.secondary)
              Text(service.config.kind == "static" ? "Static folder" : "Server command")
            }
            if service.config.kind != "static" {
              GridRow {
                Text("Command").foregroundStyle(.secondary)
                Text(service.config.command ?? "Not set").font(.body.monospaced())
              }
            }
            GridRow {
              Text("Port").foregroundStyle(.secondary)
              Text(service.config.port.map(String.init) ?? "Not set")
            }
            if let domain = service.config.domain {
              GridRow {
                Text("Domain").foregroundStyle(.secondary)
                Text(domain)
              }
            }
            GridRow {
              Text("Readiness mode").foregroundStyle(.secondary)
              Text(service.config.readiness ?? "port")
            }
            if let status = service.report?.status {
              GridRow {
                Text("Port listening").foregroundStyle(.secondary)
                Text(status.listening.map { $0 ? "Yes" : "No" } ?? "Unknown")
              }
              if let pid = status.pid {
                GridRow {
                  Text("Process ID").foregroundStyle(.secondary)
                  Text(String(pid))
                }
              }
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(8)
          .textSelection(.enabled)
        } label: {
          Label("Configuration", systemImage: "slider.horizontal.3")
        }

        HStack {
          Text("Recent Logs").font(.headline)
          Spacer()
          Button("Diagnose", systemImage: "stethoscope") {
            Task { await store.diagnose(name: service.name) }
          }
          .disabled(store.isBusy)
          Button("Reveal Log", systemImage: "folder") {
            if let path = service.report?.status.logPath {
              NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
            }
          }
          .disabled(service.report == nil)
        }

        GroupBox {
          Text(logText)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, minHeight: 160, alignment: .topLeading)
            .padding(8)
        }

        Text("Sites run independently of this app. Quitting Preside leaves their servers running.")
          .font(.caption)
          .foregroundStyle(.secondary)
      }
      .padding(28)
    }
    .sheet(item: $renamingSite) { site in
      RenameSiteView(store: store, service: site)
    }
    .confirmationDialog(
      "Remove Site?", isPresented: $showingRemoveConfirmation,
      titleVisibility: .visible, presenting: removalSite
    ) { site in
      Button("Remove Site", role: .destructive) {
        let name = site.name
        Task { _ = await store.remove(name: name) }
      }
      Button("Cancel", role: .cancel) {}
    } message: { site in
      Text("This removes \(site.name) from Preside. Your project files are kept.")
    }
  }

  private var logText: String {
    let lines = service.report?.status.recentLogLines ?? []
    return lines.isEmpty ? "No recent output." : lines.joined(separator: "\n")
  }
}

#Preview {
  ServiceDetailView(
    store: ServiceStore(),
    service: ServiceEntry(
      name: "website",
      config: ServiceConfig(dir: "~/Developer/website", command: "npm run dev", port: 3000)
    )
  )
  .frame(width: 700, height: 640)
}
