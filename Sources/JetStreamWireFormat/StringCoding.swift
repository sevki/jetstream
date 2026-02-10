// JetStream WireFormat — String Encoding
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.string]
// r[impl jetstream.wireformat.swift.string]
// r[impl jetstream.wireformat.swift.9p-compat]
// r[impl jetstream.wireformat.9p-compat]
extension String: WireFormat {
    public func byteSize() -> UInt32 {
        UInt32(2 + self.utf8.count)
    }

    public func encode(writer: inout BinaryWriter) throws {
        let utf8Bytes = Array(self.utf8)
        guard utf8Bytes.count <= Int(UInt16.max) else {
            throw WireFormatError.stringTooLong(utf8Bytes.count)
        }
        try UInt16(utf8Bytes.count).encode(writer: &writer)
        writer.writeBytes(Data(utf8Bytes))
    }

    public static func decode(reader: inout BinaryReader) throws -> String {
        let length = try UInt16.decode(reader: &reader)
        let bytes = try reader.readBytes(count: Int(length))
        guard let string = String(data: bytes, encoding: .utf8) else {
            throw WireFormatError.invalidUTF8
        }
        return string
    }
}
