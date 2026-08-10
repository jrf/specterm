// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "specterm-tap",
    platforms: [.macOS(.v13)],
    targets: [
        .target(
            name: "SpectermTapCore",
            path: "Sources/SpectermTapCore"
        ),
        .executableTarget(
            name: "specterm-tap",
            dependencies: ["SpectermTapCore"],
            path: "Sources",
            exclude: ["SpectermTapCore"],
            linkerSettings: [
                .linkedFramework("ScreenCaptureKit"),
                .linkedFramework("CoreMedia"),
                .linkedFramework("AVFoundation"),
            ]
        ),
        .testTarget(
            name: "SpectermTapTests",
            dependencies: ["SpectermTapCore"]
        ),
    ]
)
