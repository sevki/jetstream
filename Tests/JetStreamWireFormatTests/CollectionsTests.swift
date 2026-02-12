// JetStream WireFormat — Collection Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

final class CollectionsTests: XCTestCase {

    // MARK: - Array

    // r[verify jetstream.wireformat.swift.array]
    // r[verify jetstream.wireformat.vec]
    func testEmptyArray() throws {
        let value: [UInt32] = []
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try [UInt32].decode(reader: &reader)
        XCTAssertEqual(decoded, [])
    }

    func testU32Array() throws {
        let value: [UInt32] = [1, 2, 3, 0xDEADBEEF]
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try [UInt32].decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testStringArray() throws {
        let value: [String] = ["hello", "world", ""]
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try [String].decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testArrayByteSize() {
        let value: [UInt32] = [1, 2, 3]
        XCTAssertEqual(value.byteSize(), 2 + 3 * 4) // u16 count + 3 * u32
    }

    func testArrayEncoding() throws {
        let value: [UInt8] = [0xAA, 0xBB]
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        // u16 count (2, LE) + bytes
        XCTAssertEqual(writer.data, Data([0x02, 0x00, 0xAA, 0xBB]))
    }

    // MARK: - WireData

    // r[verify jetstream.wireformat.swift.data]
    // r[verify jetstream.wireformat.data]
    func testEmptyData() throws {
        let value = WireData()
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try WireData.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testDataRoundTrip() throws {
        let value = WireData(Data([0x01, 0x02, 0x03, 0x04, 0x05]))
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try WireData.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testDataByteSize() {
        let value = WireData(Data([1, 2, 3]))
        XCTAssertEqual(value.byteSize(), 4 + 3) // u32 length + 3 bytes
    }

    func testDataEncoding() throws {
        let value = WireData(Data([0xAA, 0xBB]))
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        // u32 length (2, LE) + bytes
        XCTAssertEqual(writer.data, Data([0x02, 0x00, 0x00, 0x00, 0xAA, 0xBB]))
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testDataTooLarge() throws {
        // Craft a header claiming more than 32MB
        var writer = BinaryWriter()
        writer.writeU32(33_554_433) // 32MB + 1
        var reader = BinaryReader(data: writer.data)
        XCTAssertThrowsError(try WireData.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.dataTooLarge(33_554_433))
        }
    }

    func testDataUnexpectedEOF() throws {
        // Header says 10 bytes, but only 3 available
        var writer = BinaryWriter()
        writer.writeU32(10)
        writer.writeU8(0x01)
        writer.writeU8(0x02)
        writer.writeU8(0x03)
        var reader = BinaryReader(data: writer.data)
        XCTAssertThrowsError(try WireData.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.unexpectedEOF)
        }
    }

    // MARK: - OrderedMap

    // r[verify jetstream.wireformat.swift.dict]
    // r[verify jetstream.wireformat.map]
    func testEmptyMap() throws {
        let value = OrderedMap<String, UInt32>()
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try OrderedMap<String, UInt32>.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testMapRoundTrip() throws {
        let value = OrderedMap<String, UInt32>([
            (key: "alpha", value: 1),
            (key: "beta", value: 2),
            (key: "gamma", value: 3),
        ])
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try OrderedMap<String, UInt32>.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testMapSortedOrder() throws {
        // Keys should be emitted in sorted order regardless of insertion order
        let value = OrderedMap<String, UInt32>([
            (key: "zebra", value: 1),
            (key: "apple", value: 2),
        ])
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        // Read count
        let count = try UInt16.decode(reader: &reader)
        XCTAssertEqual(count, 2)
        // First key should be "apple" (sorted)
        let key1 = try String.decode(reader: &reader)
        XCTAssertEqual(key1, "apple")
    }

    // MARK: - OrderedSet

    // r[verify jetstream.wireformat.swift.set]
    // r[verify jetstream.wireformat.set]
    func testEmptySet() throws {
        let value = OrderedSet<UInt32>()
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try OrderedSet<UInt32>.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testSetRoundTrip() throws {
        let value = OrderedSet<UInt32>([3, 1, 2])
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try OrderedSet<UInt32>.decode(reader: &reader)
        // Should be sorted: [1, 2, 3]
        XCTAssertEqual(decoded.elements, [1, 2, 3])
    }
}
