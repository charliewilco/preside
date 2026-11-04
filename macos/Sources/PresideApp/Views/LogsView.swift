import SwiftUI

struct LogsView: View {
  @Bindable var store: ServiceStore
  let name: String
  @Environment(\.dismiss) private var dismiss
  @State private var automaticallyRefresh = true

  private var lineCount: Int {
    store.logOutput.isEmpty
      ? 0 : store.logOutput.split(separator: "\n", omittingEmptySubsequences: false).count
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      HStack {
        VStack(alignment: .leading, spacing: 4) {
          Text("Logs").font(.title2.bold())
          Text(name).foregroundStyle(.secondary)
        }
        Spacer()
        if store.isLoadingLogs {
          ProgressView().controlSize(.small).accessibilityLabel("Loading logs")
        }
        Button("Copy", systemImage: "doc.on.doc") { store.copyLogs() }
          .disabled(store.logOutput.isEmpty)
        Button("Export…", systemImage: "square.and.arrow.up") {
          Task { await store.exportLogs(name: name) }
        }
        .disabled(store.logOutput.isEmpty)
      }
      if let error = store.logError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.orange)
          .textSelection(.enabled)
      }
      ScrollView([.horizontal, .vertical]) {
        Text(store.logOutput.isEmpty ? "No log output yet." : store.logOutput)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .topLeading)
          .padding(12)
          .accessibilityLabel(store.logOutput.isEmpty ? "No log output yet." : store.logOutput)
      }
      .defaultScrollAnchor(.topLeading)
      .background(.background.secondary, in: RoundedRectangle(cornerRadius: 8))
      HStack {
        Toggle("Refresh automatically", isOn: $automaticallyRefresh)
          .toggleStyle(.checkbox)
        Text("\(lineCount) lines · Most recent 500 lines, up to 1 MiB")
          .font(.caption)
          .foregroundStyle(.secondary)
        Spacer()
        Button("Refresh", systemImage: "arrow.clockwise") {
          Task { await store.refreshLogs(name: name) }
        }
        .disabled(store.isLoadingLogs)
        Button("Done") { dismiss() }
          .keyboardShortcut(.defaultAction)
      }
    }
    .padding(24)
    .frame(minWidth: 820, idealWidth: 940, minHeight: 500, idealHeight: 620)
    .task(id: name) {
      store.logOutput = ""
      store.logError = nil
      await store.refreshLogs(name: name)
    }
    .task(id: "\(name):\(automaticallyRefresh)") {
      guard automaticallyRefresh else { return }
      while !Task.isCancelled {
        do { try await Task.sleep(for: .seconds(2)) } catch { return }
        await store.refreshLogs(name: name)
      }
    }
  }
}

#Preview {
  LogsView(store: ServiceStore(), name: "website")
}
