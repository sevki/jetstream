// JetStream WireFormat — String Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

final class StringTests: XCTestCase {

    func roundTrip(_ value: String) throws -> String {
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        return try String.decode(reader: &reader)
    }

    // r[verify jetstream.wireformat.swift.string]
    // r[verify jetstream.wireformat.string]
    func testEmptyString() throws {
        XCTAssertEqual(try roundTrip(""), "")
    }

    func testSimpleString() throws {
        XCTAssertEqual(try roundTrip("hello"), "hello")
    }

    func testUTF8String() throws {
        XCTAssertEqual(try roundTrip("hello 世界 🌍"), "hello 世界 🌍")
    }

    func testStringEncoding() throws {
        var writer = BinaryWriter()
        try "hi".encode(writer: &writer)
        // u16 length (2, LE) + "hi" bytes
        XCTAssertEqual(writer.data, Data([0x02, 0x00, 0x68, 0x69]))
    }

    func testStringByteSize() {
        XCTAssertEqual("".byteSize(), 2) // just the u16 length prefix
        XCTAssertEqual("hi".byteSize(), 4) // 2 + 2 bytes
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testInvalidUTF8() throws {
        // Construct data with length 2 but invalid UTF-8 bytes
        var writer = BinaryWriter()
        writer.writeU16(2)
        writer.writeU8(0xFF)
        writer.writeU8(0xFE)
        var reader = BinaryReader(data: writer.data)
        XCTAssertThrowsError(try String.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidUTF8)
        }
    }

    // r[verify jetstream.wireformat.swift.9p-compat]
    func test9PStringFormat() throws {
        // Verify that encoding matches 9P string[s] format:
        // u16 byte count (LE) followed by UTF-8 data
        var writer = BinaryWriter()
        try "ABC".encode(writer: &writer)
        let bytes = [UInt8](writer.data)
        XCTAssertEqual(bytes[0], 3)   // length low byte
        XCTAssertEqual(bytes[1], 0)   // length high byte
        XCTAssertEqual(bytes[2], 0x41) // 'A'
        XCTAssertEqual(bytes[3], 0x42) // 'B'
        XCTAssertEqual(bytes[4], 0x43) // 'C'
    }

    func testStringEOF() throws {
        // String header says 10 bytes, but only 3 available
        var writer = BinaryWriter()
        writer.writeU16(10)
        writer.writeU8(0x41)
        writer.writeU8(0x42)
        writer.writeU8(0x43)
        var reader = BinaryReader(data: writer.data)
        XCTAssertThrowsError(try String.decode(reader: &reader))
    }
}
