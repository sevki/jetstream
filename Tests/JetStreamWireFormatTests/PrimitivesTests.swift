// JetStream WireFormat — Primitive Type Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause
// r[impl jetstream.wireformat.swift.test-roundtrip]
// r[impl jetstream.wireformat.swift.test-error]

import XCTest
@testable import JetStreamWireFormat

final class PrimitivesTests: XCTestCase {

    // MARK: - Helpers

    func roundTrip<T: WireFormat & Equatable>(_ value: T) throws -> T {
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        return try T.decode(reader: &reader)
    }

    // MARK: - UInt8

    // r[verify jetstream.wireformat.swift.u8]
    // r[verify jetstream.wireformat.u8]
    func testU8RoundTrip() throws {
        XCTAssertEqual(try roundTrip(UInt8(0)), 0)
        XCTAssertEqual(try roundTrip(UInt8(127)), 127)
        XCTAssertEqual(try roundTrip(UInt8(255)), 255)
    }

    func testU8ByteSize() {
        XCTAssertEqual(UInt8(42).byteSize(), 1)
    }

    func testU8Encoding() throws {
        var writer = BinaryWriter()
        try UInt8(0xAB).encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0xAB]))
    }

    // MARK: - UInt16

    // r[verify jetstream.wireformat.swift.u16]
    // r[verify jetstream.wireformat.u16]
    func testU16RoundTrip() throws {
        XCTAssertEqual(try roundTrip(UInt16(0)), 0)
        XCTAssertEqual(try roundTrip(UInt16(256)), 256)
        XCTAssertEqual(try roundTrip(UInt16(65535)), 65535)
    }

    func testU16LittleEndian() throws {
        var writer = BinaryWriter()
        try UInt16(0x0102).encode(writer: &writer)
        // Little-endian: low byte first
        XCTAssertEqual(writer.data, Data([0x02, 0x01]))
    }

    // MARK: - UInt32

    // r[verify jetstream.wireformat.swift.u32]
    // r[verify jetstream.wireformat.u32]
    func testU32RoundTrip() throws {
        XCTAssertEqual(try roundTrip(UInt32(0)), 0)
        XCTAssertEqual(try roundTrip(UInt32(0xDEADBEEF)), 0xDEADBEEF)
    }

    func testU32LittleEndian() throws {
        var writer = BinaryWriter()
        try UInt32(0x01020304).encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x04, 0x03, 0x02, 0x01]))
    }

    // MARK: - UInt64

    // r[verify jetstream.wireformat.swift.u64]
    // r[verify jetstream.wireformat.u64]
    func testU64RoundTrip() throws {
        XCTAssertEqual(try roundTrip(UInt64(0)), 0)
        XCTAssertEqual(try roundTrip(UInt64.max), UInt64.max)
    }

    func testU64LittleEndian() throws {
        var writer = BinaryWriter()
        try UInt64(0x0102030405060708).encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]))
    }

    // MARK: - UInt128

    // r[verify jetstream.wireformat.swift.u128]
    // r[verify jetstream.wireformat.u128]
    func testUInt128RoundTrip() throws {
        let value = WireUInt128(low: 0x0102030405060708, high: 0x090A0B0C0D0E0F10)
        XCTAssertEqual(try roundTrip(value), value)
    }

    func testUInt128Zero() throws {
        let value = WireUInt128(low: 0, high: 0)
        XCTAssertEqual(try roundTrip(value), value)
    }

    func testUInt128ByteSize() {
        XCTAssertEqual(WireUInt128(0).byteSize(), 16)
    }

    // MARK: - Int16

    // r[verify jetstream.wireformat.swift.i16]
    // r[verify jetstream.wireformat.i16]
    func testI16RoundTrip() throws {
        XCTAssertEqual(try roundTrip(Int16(0)), 0)
        XCTAssertEqual(try roundTrip(Int16(-1)), -1)
        XCTAssertEqual(try roundTrip(Int16.min), Int16.min)
        XCTAssertEqual(try roundTrip(Int16.max), Int16.max)
    }

    // MARK: - Int32

    // r[verify jetstream.wireformat.swift.i32]
    // r[verify jetstream.wireformat.i32]
    func testI32RoundTrip() throws {
        XCTAssertEqual(try roundTrip(Int32(0)), 0)
        XCTAssertEqual(try roundTrip(Int32(-1)), -1)
        XCTAssertEqual(try roundTrip(Int32.min), Int32.min)
        XCTAssertEqual(try roundTrip(Int32.max), Int32.max)
    }

    // MARK: - Int64

    // r[verify jetstream.wireformat.swift.i64]
    // r[verify jetstream.wireformat.i64]
    func testI64RoundTrip() throws {
        XCTAssertEqual(try roundTrip(Int64(0)), 0)
        XCTAssertEqual(try roundTrip(Int64(-1)), -1)
        XCTAssertEqual(try roundTrip(Int64.min), Int64.min)
        XCTAssertEqual(try roundTrip(Int64.max), Int64.max)
    }

    // MARK: - Int128

    // r[verify jetstream.wireformat.swift.i128]
    // r[verify jetstream.wireformat.i128]
    func testInt128RoundTrip() throws {
        let value = WireInt128(low: 0x0102030405060708, high: -1)
        XCTAssertEqual(try roundTrip(value), value)
    }

    func testInt128FromNegative() throws {
        let value = WireInt128(-42)
        XCTAssertEqual(try roundTrip(value), value)
    }

    // MARK: - Float

    // r[verify jetstream.wireformat.swift.f32]
    // r[verify jetstream.wireformat.f32]
    func testF32RoundTrip() throws {
        XCTAssertEqual(try roundTrip(Float(0.0)), 0.0)
        XCTAssertEqual(try roundTrip(Float(3.14)), Float(3.14))
        XCTAssertEqual(try roundTrip(Float(-1.5)), -1.5)
        XCTAssert(try roundTrip(Float.infinity) == Float.infinity)
        XCTAssert(try roundTrip(Float.nan).isNaN)
    }

    func testF32ByteSize() {
        XCTAssertEqual(Float(1.0).byteSize(), 4)
    }

    // MARK: - Double

    // r[verify jetstream.wireformat.swift.f64]
    // r[verify jetstream.wireformat.f64]
    func testF64RoundTrip() throws {
        XCTAssertEqual(try roundTrip(Double(0.0)), 0.0)
        XCTAssertEqual(try roundTrip(Double(3.141592653589793)), 3.141592653589793)
        XCTAssertEqual(try roundTrip(Double(-1.5)), -1.5)
        XCTAssert(try roundTrip(Double.infinity) == Double.infinity)
        XCTAssert(try roundTrip(Double.nan).isNaN)
    }

    func testF64ByteSize() {
        XCTAssertEqual(Double(1.0).byteSize(), 8)
    }

    // MARK: - Bool

    // r[verify jetstream.wireformat.swift.bool]
    // r[verify jetstream.wireformat.bool]
    func testBoolRoundTrip() throws {
        XCTAssertEqual(try roundTrip(true), true)
        XCTAssertEqual(try roundTrip(false), false)
    }

    func testBoolEncoding() throws {
        var writer = BinaryWriter()
        try true.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x01]))

        var writer2 = BinaryWriter()
        try false.encode(writer: &writer2)
        XCTAssertEqual(writer2.data, Data([0x00]))
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testBoolInvalidByte() throws {
        var reader = BinaryReader(data: Data([0x02]))
        XCTAssertThrowsError(try Bool.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidBoolByte(2))
        }
    }

    // MARK: - Unit

    // r[verify jetstream.wireformat.swift.unit]
    // r[verify jetstream.wireformat.unit]
    func testUnitRoundTrip() throws {
        let value = WireUnit()
        var writer = BinaryWriter()
        try value.encode(writer: &writer)
        XCTAssertEqual(writer.data.count, 0)

        var reader = BinaryReader(data: Data())
        let decoded = try WireUnit.decode(reader: &reader)
        XCTAssertEqual(decoded, value)
    }

    func testUnitByteSize() {
        XCTAssertEqual(WireUnit().byteSize(), 0)
    }

    // MARK: - EOF tests

    // r[verify jetstream.wireformat.swift.test-error]
    func testUnexpectedEOF() throws {
        // Try to read a u32 from only 2 bytes
        var reader = BinaryReader(data: Data([0x01, 0x02]))
        XCTAssertThrowsError(try UInt32.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.unexpectedEOF)
        }
    }
}
