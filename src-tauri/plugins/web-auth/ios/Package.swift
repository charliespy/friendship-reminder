// swift-tools-version:5.5

import PackageDescription

let package = Package(
  name: "tauri-plugin-web-auth",
  platforms: [
    .macOS(.v10_13),
    .iOS(.v13),
  ],
  products: [
    .library(
      name: "tauri-plugin-web-auth",
      type: .static,
      targets: ["tauri-plugin-web-auth"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-web-auth",
      dependencies: [
        .byName(name: "Tauri")
      ],
      path: "Sources")
  ]
)
