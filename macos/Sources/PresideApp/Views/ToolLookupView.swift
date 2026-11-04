import SwiftUI

struct ToolLookupView: View {
  let searchPath: String
  @State private var tool = "node"
  @State private var checkedTool: String?
  @State private var resolvedPath: String?

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        TextField("Tool", text: $tool, prompt: Text("node, pnpm, python3, or an absolute path"))
          .onChange(of: tool) { checkedTool = nil }
        Button("Check") {
          checkedTool = tool
          resolvedPath = SetupAssistance.executable(tool: tool, searchPath: searchPath)
        }
        .disabled(tool.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
      }
      if let checkedTool {
        if let resolvedPath {
          Label(resolvedPath, systemImage: "checkmark.circle")
            .foregroundStyle(.green)
            .textSelection(.enabled)
        } else {
          Label(
            "\(checkedTool) wasn’t found. Add its executable directory to PATH, or use its absolute path in your command.",
            systemImage: "exclamationmark.triangle"
          )
          .foregroundStyle(.orange)
        }
      }
      Text(
        "Checks absolute directories in Preside’s PATH without running the executable. Shell aliases and functions are not available."
      )
      .font(.caption)
      .foregroundStyle(.secondary)
    }
    .onChange(of: searchPath) { checkedTool = nil }
  }
}

#Preview {
  ToolLookupView(searchPath: "/usr/bin:/bin")
    .padding()
    .frame(width: 520)
}
