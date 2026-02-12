// JetStream RPC — Protocol Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[verify jetstream.rpc.swift.protocol]

import XCTest
import JetStreamWireFormat
@testable import JetStreamRpc

// MARK: - Test Framer types

struct EchoRequest: Framer, Equatable {
    var value: UInt32

    func messageType() -> UInt8 { MESSAGE_ID_START }
    func byteSize() -> UInt32 { 4 }
    func encode(writer: inout BinaryWriter) throws { writer.writeU32(value) }
    static func decode(reader: inout BinaryReader, type: UInt8) throws -> EchoRequest {
        EchoRequest(value: try reader.readU32())
    }
}

struct EchoResponse: Framer, Equatable {
    var value: UInt32

    func messageType() -> UInt8 { MESSAGE_ID_START + 1 }
    func byteSize() -> UInt32 { 4 }
    func encode(writer: inout BinaryWriter) throws { writer.writeU32(value) }
    static func decode(reader: inout BinaryReader, type: UInt8) throws -> EchoResponse {
        EchoResponse(value: try reader.readU32())
    }
}

// MARK: - Test Protocol conformance

enum EchoError: Swift.Error, Sendable {
    case unknown
}

struct EchoService: JetStreamProtocol {
    typealias Request = EchoRequest
    typealias Response = EchoResponse
    typealias Error = EchoError
    static let VERSION = "1.0.0"
    static let NAME = "echo"
}

// MARK: - Tests

final class ProtocolTests: XCTestCase {

    func testProtocolVersion() {
        XCTAssertEqual(EchoService.VERSION, "1.0.0")
    }

    func testProtocolName() {
        XCTAssertEqual(EchoService.NAME, "echo")
    }

    func testProtocolAssociatedTypes() {
        // Verify associated types resolve correctly
        let _: EchoService.Request.Type = EchoRequest.self
        let _: EchoService.Response.Type = EchoResponse.self
        let _: EchoService.Error.Type = EchoError.self
    }
}
