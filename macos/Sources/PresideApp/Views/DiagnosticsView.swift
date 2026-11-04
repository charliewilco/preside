import SwiftUI

struct DiagnosticsView: View {
  let output: String
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Label("Site Diagnostics", systemImage: "stethoscope")
        .font(.title2.bold())
      ScrollView {
        Text(output)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding()
      }
      .background(.background.secondary)
      HStack {
        Text("Diagnostics only. No automatic repairs were applied.")
          .font(.caption)
          .foregroundStyle(.secondary)
        Spacer()
        Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
      }
    }
    .padding(24)
    .frame(width: 660, height: 460)
  }
}

#Preview {
  DiagnosticsView(output: "example\n  config: ok\n  process: stopped\n  port: not listening")
}
