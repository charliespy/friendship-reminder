// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Friends",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "Friends",
            path: "Sources/Friends",
            exclude: ["Resources/Info.plist"]
        ),
        .testTarget(
            name: "FriendsTests",
            dependencies: ["Friends"],
            path: "Tests/FriendsTests"
        )
    ]
)
