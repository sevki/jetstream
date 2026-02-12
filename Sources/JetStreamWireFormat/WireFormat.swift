// JetStream WireFormat Protocol
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.trait]
// r[impl jetstream.wireformat.swift.protocol]
/// A type that can be encoded on the wire using the JetStream/9P2000.L wire format.
public protocol WireFormat {
    /// Returns the number of bytes required to encode this value.
    func byteSize() -> UInt32

    /// Encodes this value into the given writer.
    func encode(writer: inout BinaryWriter) throws

    /// Decodes a value of this type from the given reader.
    static func decode(reader: inout BinaryReader) throws -> Self
}

/// Errors that can occur during wire format encoding/decoding.
public enum WireFormatError: Error, Equatable {
    case unexpectedEOF
    case stringTooLong(Int)
    case dataTooLarge(UInt32)
    case invalidBoolByte(UInt8)
    case invalidOptionalTag(UInt8)
    case invalidEnumVariant(UInt8)
    case invalidLevelValue(UInt8)
    case invalidIPAddressTag(UInt8)
    case invalidSocketAddrTag(UInt8)
    case invalidUTF8
    case timestampOverflow
    case tooManyElements(Int)
}
