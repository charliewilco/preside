import SwiftUI

struct RenameSiteView: View {
  @Bindable var store: ServiceStore
  let service: ServiceEntry
  @Environment(\.dismiss) private var dismiss
  @State private var name: String

  init(store: ServiceStore, service: ServiceEntry) {
    self.store = store
    self.service = service
    _name = State(initialValue: service.name)
  }

  private var validationError: String? {
    if name.range(of: "^[A-Za-z0-9][A-Za-z0-9_.-]*$", options: .regularExpression) == nil {
      return "Start with a letter or number; use letters, numbers, dots, underscores, or hyphens."
    }
    if name != service.name && store.services.contains(where: { $0.name == name }) {
      return "A site with this name already exists."
    }
    return nil
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Rename Site").font(.title2.bold())
      Text("Choose a new name for \(service.name). Your project files stay in place.")
        .foregroundStyle(.secondary)
      TextField("Name", text: $name)
        .disabled(store.isBusy)
      if let validationError {
        Text(validationError).font(.callout).foregroundStyle(.secondary)
      }
      if let error = store.error {
        Text(error).font(.callout).foregroundStyle(.red).textSelection(.enabled)
      }
      HStack {
        Spacer()
        Button("Cancel") { dismiss() }
          .keyboardShortcut(.cancelAction)
          .disabled(store.isBusy)
        Button("Rename") {
          Task {
            if await store.rename(name: service.name, to: name) { dismiss() }
          }
        }
        .keyboardShortcut(.defaultAction)
        .disabled(store.isBusy || validationError != nil || name == service.name)
      }
    }
    .padding(24)
    .frame(width: 460)
    .interactiveDismissDisabled(store.isBusy)
  }
}

#Preview {
  RenameSiteView(
    store: ServiceStore(),
    service: ServiceEntry(
      name: "website",
      config: ServiceConfig(dir: "~/Developer/website", command: "npm run dev", port: 3000)
    )
  )
}
