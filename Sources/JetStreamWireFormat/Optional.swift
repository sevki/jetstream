// JetStream WireFormat — Optional Encoding
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.option]
// r[impl jetstream.wireformat.swift.optional]

/// Helper to encode an Optional value using the wire format.
/// Since Swift doesn't allow direct protocol conformance on Optional generically
/// with associated type constraints, we provide free functions.
public enum OptionalCoding {
    public static func byteSize<T: WireFormat>(_ value: T?) -> UInt32 {
        switch value {
        case .none:
            return 1
        case .some(let inner):
            return 1 + inner.byteSize()
        }
    }

    public static func encode<T: WireFormat>(_ value: T?, writer: inout BinaryWriter) throws {
        switch value {
        case .none:
            writer.writeU8(0)
        case .some(let inner):
            writer.writeU8(1)
            try inner.encode(writer: &writer)
        }
    }

    public static func decode<T: WireFormat>(reader: inout BinaryReader) throws -> T? {
        let tag = try reader.readU8()
        switch tag {
        case 0:
            return nil
        case 1:
            return try T.decode(reader: &reader)
        default:
            throw WireFormatError.invalidOptionalTag(tag)
        }
    }
}

/// WireFormat conformance for Optional where Wrapped conforms to WireFormat.
extension Optional: WireFormat where Wrapped: WireFormat {
    public func byteSize() -> UInt32 {
        OptionalCoding.byteSize(self)
    }

    public func encode(writer: inout BinaryWriter) throws {
        try OptionalCoding.encode(self, writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Wrapped? {
        try OptionalCoding.decode(reader: &reader)
    }
}
