import AppKit
import SwiftUI

struct AddServiceView: View {
  @Bindable var store: ServiceStore
  @Environment(\.dismiss) private var dismiss
  @State private var draft = ServiceDraft()

  private var isStatic: Bool { draft.kind == "static" }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Add Site").font(.title2.bold())
      Text("Choose a static folder or a command that starts your development server.")
        .foregroundStyle(.secondary)
      Form {
        Section {
          Picker("Site type", selection: $draft.kind) {
            Text("Static folder").tag("static")
            Text("Server command").tag("command")
          }
          .pickerStyle(.segmented)
          .accessibilityLabel("Site type")
          if isStatic {
            Text(
              "Preside serves this folder on your Mac using its built-in server. No command or extra tools are needed."
            )
            .font(.callout)
            .foregroundStyle(.secondary)
          } else {
            Text(
              "Preside runs your command from the project folder with your Mac user account’s permissions."
            )
            .font(.callout)
            .foregroundStyle(.secondary)
          }
          TextField("Name", text: $draft.name, prompt: Text("my-site"))
          HStack {
            TextField(
              isStatic ? "Site folder" : "Project folder", text: $draft.dir,
              prompt: Text("~/Developer/my-site"))
            Button("Choose…", action: chooseDirectory)
          }
          if !isStatic {
            TextField("Command", text: $draft.command, prompt: Text("npm run dev"))
          }
          TextField("Port", text: $draft.port, prompt: Text("3000"))
        }
        if !isStatic {
          Section("Readiness") {
            Picker("Check", selection: $draft.readiness) {
              Text("Process stays running").tag("process")
              Text("Port accepts connections").tag("port")
              Text("HTTP returns success").tag("http")
            }
            if draft.readiness == "http" {
              TextField("Health path", text: $draft.healthPath, prompt: Text("/healthz"))
            }
          }
        }
        Section("Optional") {
          TextField("Domain", text: $draft.domain, prompt: Text("my-site.local"))
          Text(
            "Domains must already resolve on your Mac. Preside does not configure DNS or a proxy."
          )
          .font(.caption)
          .foregroundStyle(.secondary)
          if !isStatic {
            VStack(alignment: .leading, spacing: 6) {
              Text("Environment variables")
              TextEditor(text: $draft.environment)
                .accessibilityLabel("Environment variables")
                .accessibilityHint("One KEY=value per line")
                .font(.system(.body, design: .monospaced))
                .frame(height: 70)
                .border(.separator)
              Text("One KEY=value per line. Values are saved in your Preside configuration.")
                .font(.caption)
                .foregroundStyle(.secondary)
            }
          }
        }
      }
      .formStyle(.grouped)
      .disabled(store.isBusy)

      if let error = store.error {
        Text(error).foregroundStyle(.red).font(.callout).textSelection(.enabled)
      }
      Text("Adding a site saves its settings. Choose Start when you’re ready to run it.")
        .font(.caption)
        .foregroundStyle(.secondary)
      HStack {
        if let error = draft.validationError {
          Text(error).font(.caption).foregroundStyle(.secondary)
        }
        Spacer()
        Button("Cancel") { dismiss() }
          .keyboardShortcut(.cancelAction)
          .disabled(store.isBusy)
        Button("Add Site") {
          Task { if await store.add(draft) { dismiss() } }
        }
        .keyboardShortcut(.defaultAction)
        .disabled(store.isBusy || draft.validationError != nil)
      }
    }
    .padding(24)
    .frame(width: 600, height: 690)
    .interactiveDismissDisabled(store.isBusy)
  }

  private func chooseDirectory() {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    if panel.runModal() == .OK, let url = panel.url {
      draft.dir = url.path
    }
  }
}

#Preview {
  AddServiceView(store: ServiceStore())
}
