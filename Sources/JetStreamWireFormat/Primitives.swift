// JetStream WireFormat — Primitive Types
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// MARK: - UInt8

// r[impl jetstream.wireformat.u8]
// r[impl jetstream.wireformat.swift.u8]
extension UInt8: WireFormat {
    public func byteSize() -> UInt32 { 1 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU8(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> UInt8 {
        try reader.readU8()
    }
}

// MARK: - UInt16

// r[impl jetstream.wireformat.u16]
// r[impl jetstream.wireformat.swift.u16]
extension UInt16: WireFormat {
    public func byteSize() -> UInt32 { 2 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU16(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> UInt16 {
        try reader.readU16()
    }
}

// MARK: - UInt32

// r[impl jetstream.wireformat.u32]
// r[impl jetstream.wireformat.swift.u32]
extension UInt32: WireFormat {
    public func byteSize() -> UInt32 { 4 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU32(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> UInt32 {
        try reader.readU32()
    }
}

// MARK: - UInt64

// r[impl jetstream.wireformat.u64]
// r[impl jetstream.wireformat.swift.u64]
extension UInt64: WireFormat {
    public func byteSize() -> UInt32 { 8 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU64(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> UInt64 {
        try reader.readU64()
    }
}

// MARK: - UInt128 (custom struct for Swift <6 compatibility)

// r[impl jetstream.wireformat.u128]
// r[impl jetstream.wireformat.swift.u128]
/// A 128-bit unsigned integer for wire format encoding.
/// Stored as (low: UInt64, high: UInt64). Encoded low-first (little-endian).
public struct WireUInt128: Equatable, Hashable {
    public var low: UInt64
    public var high: UInt64

    public init(low: UInt64, high: UInt64) {
        self.low = low
        self.high = high
    }

    public init(_ value: UInt64) {
        self.low = value
        self.high = 0
    }
}

extension WireUInt128: WireFormat {
    public func byteSize() -> UInt32 { 16 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU64(low)
        writer.writeU64(high)
    }

    public static func decode(reader: inout BinaryReader) throws -> WireUInt128 {
        let low = try reader.readU64()
        let high = try reader.readU64()
        return WireUInt128(low: low, high: high)
    }
}

// MARK: - Int16

// r[impl jetstream.wireformat.i16]
// r[impl jetstream.wireformat.swift.i16]
extension Int16: WireFormat {
    public func byteSize() -> UInt32 { 2 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeI16(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> Int16 {
        try reader.readI16()
    }
}

// MARK: - Int32

// r[impl jetstream.wireformat.i32]
// r[impl jetstream.wireformat.swift.i32]
extension Int32: WireFormat {
    public func byteSize() -> UInt32 { 4 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeI32(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> Int32 {
        try reader.readI32()
    }
}

// MARK: - Int64

// r[impl jetstream.wireformat.i64]
// r[impl jetstream.wireformat.swift.i64]
extension Int64: WireFormat {
    public func byteSize() -> UInt32 { 8 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeI64(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> Int64 {
        try reader.readI64()
    }
}

// MARK: - Int128 (custom struct for Swift <6 compatibility)

// r[impl jetstream.wireformat.i128]
// r[impl jetstream.wireformat.swift.i128]
/// A 128-bit signed integer for wire format encoding.
/// Stored as (low: UInt64, high: Int64). Encoded low-first (little-endian).
public struct WireInt128: Equatable, Hashable {
    public var low: UInt64
    public var high: Int64

    public init(low: UInt64, high: Int64) {
        self.low = low
        self.high = high
    }

    public init(_ value: Int64) {
        self.low = UInt64(bitPattern: value)
        self.high = value < 0 ? -1 : 0
    }
}

extension WireInt128: WireFormat {
    public func byteSize() -> UInt32 { 16 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU64(low)
        writer.writeI64(high)
    }

    public static func decode(reader: inout BinaryReader) throws -> WireInt128 {
        let low = try reader.readU64()
        let high = try reader.readI64()
        return WireInt128(low: low, high: high)
    }
}

// MARK: - Float

// r[impl jetstream.wireformat.f32]
// r[impl jetstream.wireformat.swift.f32]
extension Float: WireFormat {
    public func byteSize() -> UInt32 { 4 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeF32(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> Float {
        try reader.readF32()
    }
}

// MARK: - Double

// r[impl jetstream.wireformat.f64]
// r[impl jetstream.wireformat.swift.f64]
extension Double: WireFormat {
    public func byteSize() -> UInt32 { 8 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeF64(self)
    }

    public static func decode(reader: inout BinaryReader) throws -> Double {
        try reader.readF64()
    }
}

// MARK: - Bool

// r[impl jetstream.wireformat.bool]
// r[impl jetstream.wireformat.swift.bool]
extension Bool: WireFormat {
    public func byteSize() -> UInt32 { 1 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU8(self ? 1 : 0)
    }

    public static func decode(reader: inout BinaryReader) throws -> Bool {
        let byte = try reader.readU8()
        switch byte {
        case 0: return false
        case 1: return true
        default: throw WireFormatError.invalidBoolByte(byte)
        }
    }
}

// MARK: - Unit (Void equivalent)

// r[impl jetstream.wireformat.unit]
// r[impl jetstream.wireformat.swift.unit]
/// The unit type, encoding zero bytes on the wire.
public struct WireUnit: Equatable, WireFormat {
    public init() {}

    public func byteSize() -> UInt32 { 0 }

    public func encode(writer: inout BinaryWriter) throws {}

    public static func decode(reader: inout BinaryReader) throws -> WireUnit {
        WireUnit()
    }
}
