// r[impl jetstream.interop.swift.types]
// Generated interop types for cross-language wire format verification.
// These mirror the Rust types in tests/jetstream_interop/src/lib.rs.

import Foundation
import JetStreamWireFormat

public struct Point: WireFormat {
    public var x: UInt32
    public var y: UInt32

    public func byteSize() -> UInt32 {
        return x.byteSize() + y.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try x.encode(writer: &writer)
        try y.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Point {
        let x = try UInt32.decode(reader: &reader)
        let y = try UInt32.decode(reader: &reader)
        return Point(x: x, y: y)
    }
}

public struct ColorPoint: WireFormat {
    public var x: UInt32
    public var y: UInt32
    public var color: String

    public func byteSize() -> UInt32 {
        return x.byteSize() + y.byteSize() + color.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try x.encode(writer: &writer)
        try y.encode(writer: &writer)
        try color.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> ColorPoint {
        let x = try UInt32.decode(reader: &reader)
        let y = try UInt32.decode(reader: &reader)
        let color = try String.decode(reader: &reader)
        return ColorPoint(x: x, y: y, color: color)
    }
}

public enum Shape: WireFormat {
    case circle(UInt32)
    case rectangle(width: UInt32, height: UInt32)

    public func byteSize() -> UInt32 {
        switch self {
        case .circle(let v): return 1 + v.byteSize()
        case .rectangle(let width, let height): return 1 + width.byteSize() + height.byteSize()
        }
    }

    public func encode(writer: inout BinaryWriter) throws {
        switch self {
        case .circle(let v):
            writer.writeU8(0)
            try v.encode(writer: &writer)
        case .rectangle(let width, let height):
            writer.writeU8(1)
            try width.encode(writer: &writer)
            try height.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> Shape {
        let variant = try reader.readU8()
        switch variant {
        case 0: return .circle(try UInt32.decode(reader: &reader))
        case 1:
            let width = try UInt32.decode(reader: &reader)
            let height = try UInt32.decode(reader: &reader)
            return .rectangle(width: width, height: height)
        default: throw WireFormatError.invalidEnumVariant(variant)
        }
    }
}

public struct Message: WireFormat {
    public var id: UInt32
    public var tags: [String]
    public var payload: String?

    public func byteSize() -> UInt32 {
        return id.byteSize() + tags.byteSize() + payload.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try id.encode(writer: &writer)
        try tags.encode(writer: &writer)
        try payload.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Message {
        let id = try UInt32.decode(reader: &reader)
        let tags = try [String].decode(reader: &reader)
        let payload = try String?.decode(reader: &reader)
        return Message(id: id, tags: tags, payload: payload)
    }
}
