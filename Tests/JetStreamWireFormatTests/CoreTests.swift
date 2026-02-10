// JetStream WireFormat — Core Abstraction Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

// r[verify jetstream.wireformat.swift.package]
// r[verify jetstream.wireformat.swift.protocol]
// r[verify jetstream.wireformat.swift.reader]
// r[verify jetstream.wireformat.swift.writer]
// r[verify jetstream.wireformat.swift.struct]
// r[verify jetstream.wireformat.swift.enum]
final class CoreTests: XCTestCase {

    // MARK: - WireFormat Protocol

    func testProtocolConformance() throws {
        // Verify UInt32 conforms to WireFormat
        let value: UInt32 = 42
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        XCTAssertEqual(value.byteSize(), 4)
        var reader = BinaryReader(data: writer.data)
        let decoded = try UInt32.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    // MARK: - BinaryReader

    func testReaderBasics() throws {
        let data = Data([0x01, 0x02, 0x03, 0x04])
        var reader = BinaryReader(data: data)
        XCTAssertEqual(reader.remaining, 4)
        let byte = try reader.readU8()
        XCTAssertEqual(byte, 0x01)
        XCTAssertEqual(reader.remaining, 3)
        XCTAssertEqual(reader.offset, 1)
    }

    func testReaderReadBytes() throws {
        let data = Data([0xAA, 0xBB, 0xCC])
        var reader = BinaryReader(data: data)
        let bytes = try reader.readBytes(count: 2)
        XCTAssertEqual(bytes, Data([0xAA, 0xBB]))
        XCTAssertEqual(reader.remaining, 1)
    }

    func testReaderEOF() throws {
        var reader = BinaryReader(data: Data())
        XCTAssertThrowsError(try reader.readU8())
    }

    // MARK: - BinaryWriter

    func testWriterBasics() throws {
        var writer = BinaryWriter()
        writer.writeU8(0xFF)
        writer.writeU16(0x0102)
        XCTAssertEqual(writer.data, Data([0xFF, 0x02, 0x01]))
    }

    func testWriterWriteBytes() throws {
        var writer = BinaryWriter()
        writer.writeBytes(Data([0x01, 0x02, 0x03]))
        XCTAssertEqual(writer.data.count, 3)
    }

    // MARK: - Struct encoding (using ErrorInner as example)

    func testStructFieldOrder() throws {
        // ErrorInner is a struct encoded per WF-STRUCT: fields in declaration order
        let inner = ErrorInner(message: "test", code: "E1", help: nil, url: nil)
        var writer = BinaryWriter()
        try inner.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try ErrorInner.decode(reader: &reader)
        XCTAssertEqual(decoded, inner)
        XCTAssertEqual(reader.remaining, 0)
    }

    // MARK: - Enum encoding (using IpAddr as example)

    func testEnumVariantTag() throws {
        // IpAddr uses custom tags (4, 6) not standard 0-based
        let addr = IpAddr.v4(IPv4Address(127, 0, 0, 1))
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        // First byte should be the tag
        XCTAssertEqual([UInt8](writer.data)[0], 4)
    }

    func testEnumRoundTrip() throws {
        let addr = IpAddr.v6(IPv6Address.loopback)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try IpAddr.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testEnumInvalidVariant() throws {
        // Tag 5 is invalid for IpAddr
        var reader = BinaryReader(data: Data([5]))
        XCTAssertThrowsError(try IpAddr.decode(reader: &reader))
    }
}
