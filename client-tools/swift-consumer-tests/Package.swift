// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MedTrackerSwiftConsumerTests",
    platforms: [
        .macOS(.v14),
    ],
    dependencies: [
        .package(path: "../../tmp/api-clients/swift"),
    ],
    targets: [
        .testTarget(
            name: "MedTrackerAPIConsumerTests",
            dependencies: [
                .product(name: "MedTrackerAPI", package: "swift"),
            ]
        ),
    ]
)
