// JetStream RPC — Frame Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[verify jetstream.rpc.swift.frame]
// r[verify jetstream.rpc.swift.framer]

import XCTest
@testable import JetStreamWireFormat
@testable import JetStreamRpc

/// A simple test message that implements Framer.
struct TestMessage: Framer, Equatable {
    var typeId: UInt8
    var value: UInt32

    func messageType() -> UInt8 { typeId }

    func byteSize() -> UInt32 { 4 }

    func encode(writer: inout BinaryWriter) throws {
        writer.writeU32(value)
    }

    static func decode(reader: inout BinaryReader, type: UInt8) throws -> TestMessage {
        let value = try reader.readU32()
        return TestMessage(typeId: type, value: value)
    }
}

final class FrameTests: XCTestCase {

    // MARK: - Round-trip

    func testFrameRoundTrip() throws {
        let msg = TestMessage(typeId: 101, value: 0xDEADBEEF)
        let frame = Frame(tag: 42, msg: msg)

        var writer = BinaryWriter()
        try frame.encode(writer: &writer)

        var reader = BinaryReader(data: writer.data)
        let decoded = try Frame<TestMessage>.decode(reader: &reader)

        XCTAssertEqual(decoded.tag, 42)
        XCTAssertEqual(decoded.msg, msg)
    }

    // MARK: - Byte-level encoding

    func testFrameByteLayout() throws {
        let msg = TestMessage(typeId: 7, value: 0x01020304)
        let frame = Frame(tag: 0x0A0B, msg: msg)

        var writer = BinaryWriter()
        try frame.encode(writer: &writer)
        let bytes = writer.data

        // Total size: 4 + 1 + 2 + 4 = 11 bytes
        XCTAssertEqual(bytes.count, 11)

        // size: u32 LE = 11
        XCTAssertEqual(bytes[0], 11)
        XCTAssertEqual(bytes[1], 0)
        XCTAssertEqual(bytes[2], 0)
        XCTAssertEqual(bytes[3], 0)

        // type: u8 = 7
        XCTAssertEqual(bytes[4], 7)

        // tag: u16 LE = 0x0A0B
        XCTAssertEqual(bytes[5], 0x0B)
        XCTAssertEqual(bytes[6], 0x0A)

        // payload: u32 LE = 0x01020304
        XCTAssertEqual(bytes[7], 0x04)
        XCTAssertEqual(bytes[8], 0x03)
        XCTAssertEqual(bytes[9], 0x02)
        XCTAssertEqual(bytes[10], 0x01)
    }

    // MARK: - byteSize

    func testFrameByteSize() {
        let msg = TestMessage(typeId: 1, value: 0)
        let frame = Frame(tag: 1, msg: msg)
        // 4 (size) + 1 (type) + 2 (tag) + 4 (payload) = 11
        XCTAssertEqual(frame.byteSize(), 11)
    }

    // MARK: - Minimum size validation

    func testFrameTooSmallSize() throws {
        // Manually write a frame with size < 4
        var writer = BinaryWriter()
        writer.writeU32(3) // invalid: size < 4
        writer.writeU8(1)
        writer.writeU16(1)
        writer.writeU32(0)

        var reader = BinaryReader(data: writer.data)
        XCTAssertThrowsError(try Frame<TestMessage>.decode(reader: &reader)) { error in
            if let frameError = error as? FrameError {
                XCTAssertEqual(frameError, .frameTooSmall(3))
            } else {
                XCTFail("Expected FrameError.frameTooSmall, got \(error)")
            }
        }
    }

    // MARK: - Multiple frames

    func testMultipleFramesInSequence() throws {
        let msg1 = TestMessage(typeId: 101, value: 1)
        let msg2 = TestMessage(typeId: 102, value: 2)
        let frame1 = Frame(tag: 1, msg: msg1)
        let frame2 = Frame(tag: 2, msg: msg2)

        var writer = BinaryWriter()
        try frame1.encode(writer: &writer)
        try frame2.encode(writer: &writer)

        var reader = BinaryReader(data: writer.data)
        let decoded1 = try Frame<TestMessage>.decode(reader: &reader)
        let decoded2 = try Frame<TestMessage>.decode(reader: &reader)

        XCTAssertEqual(decoded1.tag, 1)
        XCTAssertEqual(decoded1.msg.value, 1)
        XCTAssertEqual(decoded2.tag, 2)
        XCTAssertEqual(decoded2.msg.value, 2)
    }
}
