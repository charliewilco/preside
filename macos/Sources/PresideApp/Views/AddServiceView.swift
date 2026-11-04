import AppKit
import SwiftUI

struct AddServiceView: View {
  @Bindable var store: ServiceStore
  @Environment(\.dismiss) private var dismiss
  let existing: ServiceEntry?
  @State private var draft: ServiceDraft
  @State private var portSuggestionMessage: String?

  init(store: ServiceStore, existing: ServiceEntry? = nil) {
    self.store = store
    self.existing = existing
    var initialDraft = existing.map { ServiceDraft(entry: $0) } ?? ServiceDraft()
    initialDraft.prepareEnvironmentEditor()
    _draft = State(initialValue: initialDraft)
  }

  private var isStatic: Bool { draft.kind == "static" }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text(existing == nil ? "Add Site" : "Edit Site").font(.title2.bold())
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
            .disabled(existing != nil)
          HStack {
            TextField(
              isStatic ? "Site folder" : "Project folder", text: $draft.dir,
              prompt: Text("~/Developer/my-site"))
            Button("Choose…", action: chooseDirectory)
          }
          if !isStatic {
            TextField("Command", text: $draft.command, prompt: Text("npm run dev"))
          }
          HStack {
            TextField("Port", text: $draft.port, prompt: Text("3000"))
            Button("Suggest Port") {
              do {
                draft.port = String(try SetupAssistance.suggestPort())
                portSuggestionMessage = "Available now. Preside checks again when the site starts."
              } catch { portSuggestionMessage = error.localizedDescription }
            }
          }
          if let portSuggestionMessage {
            Text(portSuggestionMessage).font(.caption).foregroundStyle(.secondary)
          }
        }
        if !isStatic {
          DisclosureGroup("Check a development tool") {
            ToolLookupView(searchPath: store.searchPath)
          }
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
          if !draft.domain.isEmpty {
            Picker("Domain URL", selection: $draft.urlMode) {
              if draft.urlMode == nil {
                Text("Legacy (no port)").tag(Optional<String>.none)
              }
              Text("Direct (include port)").tag(Optional("direct"))
              Text("Existing proxy (no port)").tag(Optional("proxy"))
            }
            Text(
              draft.urlMode == "direct"
                ? "Opens http://\(draft.domain):\(draft.port). The domain must already resolve on your Mac."
                : "Opens http://\(draft.domain). An existing proxy must route this domain to your server’s port."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
          }
          Text("Preside does not configure DNS, a proxy, HTTPS, or /etc/hosts.")
            .font(.caption)
            .foregroundStyle(.secondary)
          if !isStatic {
            EnvironmentEditorView(
              variables: Binding(
                get: { draft.environmentVariables ?? [] },
                set: { draft.environmentVariables = $0 }
              )
            )
          }
        }
      }
      .formStyle(.grouped)
      .disabled(store.isBusy)

      if let error = store.error {
        Text(error).foregroundStyle(.red).font(.callout).textSelection(.enabled)
      }
      Text(
        existing == nil
          ? "Adding a site saves its settings. Choose Start when you’re ready to run it."
          : "Saving changes updates this site’s settings. Restart a running site to apply them."
      )
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
        Button(existing == nil ? "Add Site" : "Save Changes") {
          Task {
            let saved: Bool
            if let existing {
              saved = await store.update(name: existing.name, draft: draft)
            } else {
              saved = await store.add(draft)
            }
            if saved { dismiss() }
          }
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
