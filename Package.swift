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
        .library(
            name: "JetStreamRpc",
            targets: ["JetStreamRpc"]
        ),
        .executable(
            name: "JetStreamInteropHelper",
            targets: ["JetStreamInteropHelper"]
        ),
    ],
    targets: [
        .target(
            name: "JetStreamWireFormat",
            path: "Sources/JetStreamWireFormat"
        ),
        .target(
            name: "JetStreamRpc",
            dependencies: ["JetStreamWireFormat"],
            path: "Sources/JetStreamRpc"
        ),
        .executableTarget(
            name: "JetStreamInteropHelper",
            dependencies: ["JetStreamWireFormat"],
            path: "Sources/JetStreamInteropHelper"
        ),
        .testTarget(
            name: "JetStreamWireFormatTests",
            dependencies: ["JetStreamWireFormat"],
            path: "Tests/JetStreamWireFormatTests"
        ),
        .testTarget(
            name: "JetStreamRpcTests",
            dependencies: ["JetStreamRpc", "JetStreamWireFormat"],
            path: "Tests/JetStreamRpcTests"
        ),
    ]
)
