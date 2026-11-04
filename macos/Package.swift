// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "Preside",
  platforms: [.macOS(.v14)],
  products: [.executable(name: "Preside", targets: ["PresideApp"])],
  targets: [
    .executableTarget(name: "PresideApp"),
    .testTarget(name: "PresideAppTests", dependencies: ["PresideApp"]),
  ]
)
