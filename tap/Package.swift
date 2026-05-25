// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "specterm-tap",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "specterm-tap",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("ScreenCaptureKit"),
                .linkedFramework("CoreMedia"),
                .linkedFramework("AVFoundation"),
            ]
        ),
    ]
)
