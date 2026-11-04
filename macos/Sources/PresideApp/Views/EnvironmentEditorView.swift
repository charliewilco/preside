import Foundation
import SwiftUI

struct EnvironmentEditorView: View {
  @Binding var variables: [EnvironmentVariable]
  @FocusState private var focusedKey: UUID?

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text("Environment variables")
      ForEach($variables) { $variable in
        HStack(alignment: .top, spacing: 8) {
          TextField("KEY", text: $variable.key)
            .textFieldStyle(.roundedBorder)
            .font(.system(.body, design: .monospaced))
            .frame(width: 140)
            .accessibilityLabel("Environment variable name")
            .focused($focusedKey, equals: variable.id)
          TextField("Value", text: $variable.value, axis: .vertical)
            .textFieldStyle(.roundedBorder)
            .font(.system(.body, design: .monospaced))
            .lineLimit(1...5)
            .accessibilityLabel(
              variable.key.isEmpty ? "Variable value" : "Value for \(variable.key)"
            )
            .accessibilityHint("Values may contain multiple lines.")
          Button {
            let id = variable.id
            variables.removeAll { $0.id == id }
          } label: {
            Image(systemName: "minus.circle")
          }
          .buttonStyle(.borderless)
          .accessibilityLabel(
            variable.key.isEmpty ? "Remove environment variable" : "Remove \(variable.key)"
          )
          .help("Remove environment variable")
        }
      }
      Button("Add Variable", systemImage: "plus") {
        let variable = EnvironmentVariable(key: "", value: "")
        variables.append(variable)
        focusedKey = variable.id
      }
      Text(
        "Values can span multiple lines and are saved as plain text in your Preside configuration."
      )
      .font(.caption)
      .foregroundStyle(.secondary)
    }
  }
}

#Preview {
  EnvironmentEditorView(
    variables: .constant([
      EnvironmentVariable(key: "NODE_ENV", value: "development"),
      EnvironmentVariable(key: "MESSAGE", value: "First line\nSecond line"),
    ])
  )
  .padding()
  .frame(width: 520)
}
