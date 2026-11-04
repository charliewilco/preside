import AppKit
import SwiftUI

struct ServiceDetailView: View {
  @Bindable var store: ServiceStore
  let service: ServiceEntry

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
          if store.isBusy {
            ProgressView().controlSize(.small).accessibilityLabel("Running site action")
          }
          Button(
            service.canStop ? "Stop" : "Start",
            systemImage: service.canStop ? "stop.fill" : "play.fill"
          ) {
            Task { await store.perform(service.canStop ? .stop : .start, name: service.name) }
          }
          .buttonStyle(.borderedProminent)
          .disabled(store.isBusy || !(service.canStart || service.canStop))
          Button("Open", systemImage: "arrow.up.right.square") {
            Task { await store.perform(.open, name: service.name) }
          }
          .disabled(store.isBusy || service.report?.status.url == nil)
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
              "The running site uses older configuration. Stop and start it to apply your changes.",
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
