import SwiftUI

struct ServiceRowView: View {
  let service: ServiceEntry

  var body: some View {
    HStack(spacing: 10) {
      Image(systemName: service.canStop ? "play.circle.fill" : "circle")
        .foregroundStyle(service.canStop ? Color.green : Color.secondary)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 3) {
        Text(service.name).font(.headline)
        Text(service.stateLabel)
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
