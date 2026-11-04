import SwiftUI

struct ContentView: View {
  @Bindable var store: ServiceStore
  @State private var search = ""

  private var filteredServices: [ServiceEntry] {
    store.services.filter {
      search.isEmpty || $0.name.localizedCaseInsensitiveContains(search)
        || ($0.config.dir?.localizedCaseInsensitiveContains(search) ?? false)
    }
  }

  var body: some View {
    NavigationSplitView {
      List(selection: $store.selection) {
        Section("Sites") {
          ForEach(filteredServices) { service in
            ServiceRowView(service: service)
              .tag(service.name)
          }
        }
      }
      .searchable(text: $search, prompt: "Find a site")
      .navigationSplitViewColumnWidth(min: 220, ideal: 260)
      .overlay {
        if store.services.isEmpty && !store.isRefreshing {
          ContentUnavailableView(
            store.error == nil ? "No Sites" : "Sites Unavailable",
            systemImage: store.error == nil ? "server.rack" : "exclamationmark.triangle"
          )
        }
      }
      .safeAreaInset(edge: .bottom) {
        HStack {
          Text("\(store.services.count) \(store.services.count == 1 ? "site" : "sites")")
            .font(.caption)
            .foregroundStyle(.secondary)
          Spacer()
          if store.isRefreshing {
            ProgressView().controlSize(.small).accessibilityLabel("Refreshing sites")
          }
        }
        .padding(12)
      }
    } detail: {
      VStack(spacing: 0) {
        if let error = store.error {
          HStack(alignment: .top) {
            Image(systemName: "exclamationmark.triangle.fill")
              .foregroundStyle(.orange)
            Text(error).textSelection(.enabled)
            Spacer()
            Button("Dismiss", systemImage: "xmark") { store.error = nil }
              .labelStyle(.iconOnly)
              .buttonStyle(.plain)
          }
          .padding()
          .background(.orange.opacity(0.08))
        }

        if let service = store.services.first(where: { $0.name == store.selection }) {
          ServiceDetailView(store: store, service: service)
        } else {
          ContentUnavailableView {
            Label("Your Local Sites", systemImage: "server.rack")
          } description: {
            Text("Serve a static folder or run a command to start your development server.")
          } actions: {
            Button("Add Site", systemImage: "plus") { store.showingAddSite = true }
              .buttonStyle(.borderedProminent)
              .disabled(store.isBusy)
          }
        }
      }
    }
    .navigationTitle("Preside")
    .toolbar {
      ToolbarItemGroup {
        Button("Refresh", systemImage: "arrow.clockwise") {
          Task { await store.refresh() }
        }
        .disabled(store.isRefreshing || store.isBusy)
        .keyboardShortcut("r")

        Button("Add Site", systemImage: "plus") { store.showingAddSite = true }
          .disabled(store.isBusy)

        Menu("More", systemImage: "ellipsis.circle") {
          Button("Reveal Configuration in Finder") { Task { await store.revealConfig() } }
          SettingsLink()
        }
      }
    }
    .sheet(isPresented: $store.showingAddSite) { AddServiceView(store: store) }
    .sheet(
      isPresented: Binding(
        get: { store.doctorOutput != nil },
        set: { if !$0 { store.doctorOutput = nil } }
      )
    ) {
      DiagnosticsView(output: store.doctorOutput ?? "")
    }
    .frame(minWidth: 840, minHeight: 560)
  }
}

#Preview {
  ContentView(store: ServiceStore())
}
