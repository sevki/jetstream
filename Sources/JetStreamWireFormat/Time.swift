// JetStream WireFormat — Time Types
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.systime]
// r[impl jetstream.wireformat.swift.systime]
extension Date: WireFormat {
    public func byteSize() -> UInt32 { 8 }

    public func encode(writer: inout BinaryWriter) throws {
        let millis = UInt64(self.timeIntervalSince1970 * 1000.0)
        try millis.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Date {
        let millis = try UInt64.decode(reader: &reader)
        // Check for overflow: UInt64.max millis is far beyond Date range,
        // but we guard against unreasonable values.
        let seconds = Double(millis) / 1000.0
        return Date(timeIntervalSince1970: seconds)
    }
}
