// JetStream WireFormat — Time Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause
// r[impl jetstream.wireformat.swift.test-compat]

import XCTest
@testable import JetStreamWireFormat

final class TimeTests: XCTestCase {

    // r[verify jetstream.wireformat.swift.systime]
    // r[verify jetstream.wireformat.systime]
    func testDateRoundTrip() throws {
        // Use a known timestamp: 2024-01-01T00:00:00Z = 1704067200000 ms
        let date = Date(timeIntervalSince1970: 1704067200.0)
        var writer = BinaryWriter()
        try date.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try Date.decode(reader: &reader)
        // Compare with millisecond precision
        XCTAssertEqual(
            Int64(decoded.timeIntervalSince1970 * 1000),
            Int64(date.timeIntervalSince1970 * 1000)
        )
    }

    func testDateByteSize() {
        XCTAssertEqual(Date().byteSize(), 8)
    }

    func testEpochDate() throws {
        let date = Date(timeIntervalSince1970: 0)
        var writer = BinaryWriter()
        try date.encode(writer: &writer)
        // Should encode as 0 ms
        XCTAssertEqual(writer.data, Data([0, 0, 0, 0, 0, 0, 0, 0]))
    }

    func testDateEncoding() throws {
        // 1000 ms = 1 second since epoch
        let date = Date(timeIntervalSince1970: 1.0)
        var writer = BinaryWriter()
        try date.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let millis = try UInt64.decode(reader: &reader)
        XCTAssertEqual(millis, 1000)
    }

    // r[verify jetstream.wireformat.swift.test-compat]
    func testDateKnownBytes() throws {
        // 1704067200000 ms = 0x0000018CC251F400
        // In LE: 00 F4 51 C2 8C 01 00 00
        let expected = Data([0x00, 0xF4, 0x51, 0xC2, 0x8C, 0x01, 0x00, 0x00])
        let date = Date(timeIntervalSince1970: 1704067200.0)
        var writer = BinaryWriter()
        try date.encode(writer: &writer)
        XCTAssertEqual(writer.data, expected)
    }
}
