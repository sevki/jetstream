// swift-tools-version: 5.7
// r[impl jetstream.wireformat.swift.package]

import PackageDescription

let package = Package(
    name: "JetStreamWireFormat",
    platforms: [
        .macOS(.v12),
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "JetStreamWireFormat",
            targets: ["JetStreamWireFormat"]
        ),
    ],
    targets: [
        .target(
            name: "JetStreamWireFormat",
            path: "Sources/JetStreamWireFormat"
        ),
        .testTarget(
            name: "JetStreamWireFormatTests",
            dependencies: ["JetStreamWireFormat"],
            path: "Tests/JetStreamWireFormatTests"
        ),
    ]
)
