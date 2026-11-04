import AppKit
import SwiftUI

struct SettingsView: View {
  @Bindable var store: ServiceStore

  var body: some View {
    Form {
      Section("About Preside") {
        LabeledContent(
          "Version",
          value: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? "Development")
        Link(
          "Download Updates…",
          destination: URL(string: "https://github.com/charliewilco/preside/releases/latest")!)
        Text(
          "Open GitHub Releases to check available downloads. Your sites are stored separately from the app."
        )
        .font(.caption)
        .foregroundStyle(.secondary)
      }
      Section("Preside CLI") {
        HStack {
          TextField("Executable", text: $store.executablePath, prompt: Text("Use bundled pp"))
          Button("Choose…") {
            let panel = NSOpenPanel()
            panel.canChooseDirectories = false
            panel.allowsMultipleSelection = false
            if panel.runModal() == .OK, let url = panel.url {
              store.executablePath = url.path
            }
          }
        }
        Text("Leave empty to use the bundled CLI. Choose a custom pp executable for development.")
          .font(.caption)
          .foregroundStyle(.secondary)
      }
      Section("Command Environment") {
        TextField("PATH", text: $store.searchPath, axis: .vertical)
          .lineLimit(3...6)
          .font(.system(.body, design: .monospaced))
        Text(
          "Server commands inherit this search path. Add directories containing your tools, such as Node or Python. Shell profile files are not loaded."
        )
        .font(.caption)
        .foregroundStyle(.secondary)
      }
      HStack {
        Button("Reveal Configuration in Finder") { Task { await store.revealConfig() } }
        Spacer()
        Button("Refresh Sites") { Task { await store.refresh() } }
          .disabled(store.isRefreshing || store.isBusy)
      }
    }
    .formStyle(.grouped)
    .padding()
    .frame(width: 620, height: 500)
  }
}

#Preview {
  SettingsView(store: ServiceStore())
}
