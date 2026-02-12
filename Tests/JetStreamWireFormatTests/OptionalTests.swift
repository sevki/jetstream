// JetStream WireFormat — Optional Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

final class OptionalTests: XCTestCase {

    // r[verify jetstream.wireformat.swift.optional]
    // r[verify jetstream.wireformat.option]
    func testNoneRoundTrip() throws {
        let value: UInt32? = nil
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded: UInt32? = try Optional<UInt32>.decode(reader: &reader)
        XCTAssertNil(decoded)
    }

    func testSomeRoundTrip() throws {
        let value: UInt32? = 42
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded: UInt32? = try Optional<UInt32>.decode(reader: &reader)
        XCTAssertEqual(decoded, 42)
    }

    func testNoneEncoding() throws {
        let value: UInt32? = nil
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x00]))
    }

    func testSomeEncoding() throws {
        let value: UInt8? = 0xFF
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x01, 0xFF]))
    }

    func testNoneByteSize() {
        let value: UInt32? = nil
        XCTAssertEqual(value.byteSize(), 1)
    }

    func testSomeByteSize() {
        let value: UInt32? = 42
        XCTAssertEqual(value.byteSize(), 5) // 1 + 4
    }

    func testOptionalString() throws {
        let value: String? = "hello"
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded: String? = try Optional<String>.decode(reader: &reader)
        XCTAssertEqual(decoded, "hello")
    }

    func testOptionalStringNil() throws {
        let value: String? = nil
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded: String? = try Optional<String>.decode(reader: &reader)
        XCTAssertNil(decoded)
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testInvalidOptionalTag() throws {
        var reader = BinaryReader(data: Data([0x02]))
        XCTAssertThrowsError(try Optional<UInt32>.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidOptionalTag(2))
        }
    }
}
