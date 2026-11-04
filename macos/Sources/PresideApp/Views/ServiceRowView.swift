import SwiftUI

struct ServiceRowView: View {
  let service: ServiceEntry
  var action: ServiceAction? = nil

  private var status: SiteStatusPresentation {
    SiteStatusPresentation(service: service, action: action)
  }

  var body: some View {
    HStack(spacing: 10) {
      Image(systemName: status.symbol)
        .foregroundStyle(status.color)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 3) {
        Text(service.name).font(.headline)
        Text(status.title)
          .font(.caption)
          .foregroundStyle(.secondary)
      }
      Spacer()
      if let port = service.config.port {
        Text(String(port))
          .font(.caption.monospacedDigit())
          .foregroundStyle(.secondary)
      }
    }
    .padding(.vertical, 4)
    .accessibilityElement(children: .combine)
  }
}

#Preview {
  ServiceRowView(
    service: ServiceEntry(
      name: "website",
      config: ServiceConfig(dir: "~/Developer/website", command: "npm run dev", port: 3000)
    )
  )
  .padding()
  .frame(width: 260)
}
