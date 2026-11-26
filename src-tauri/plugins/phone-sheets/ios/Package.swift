// swift-tools-version:5.5

import PackageDescription

let package = Package(
  name: "tauri-plugin-phone-sheets",
  platforms: [
    .macOS(.v10_13),
    .iOS(.v15),
  ],
  products: [
    .library(
      name: "tauri-plugin-phone-sheets",
      type: .static,
      targets: ["tauri-plugin-phone-sheets"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-phone-sheets",
      dependencies: [
        .byName(name: "Tauri")
      ],
      path: "Sources")
  ]
)
