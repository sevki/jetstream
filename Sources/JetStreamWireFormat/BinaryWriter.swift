// JetStream BinaryWriter
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.swift.writer]
/// A writer that appends binary data to a growable `Data` buffer.
public struct BinaryWriter {
    public private(set) var data: Data

    /// Creates a new BinaryWriter with the given initial capacity.
    public init(capacity: Int = 256) {
        self.data = Data()
        self.data.reserveCapacity(capacity)
    }

    /// Appends raw bytes to the buffer.
    public mutating func writeBytes(_ bytes: Data) {
        data.append(bytes)
    }

    /// Writes a single UInt8.
    public mutating func writeU8(_ value: UInt8) {
        data.append(value)
    }

    // r[impl jetstream.wireformat.byte-order]
    /// Writes a UInt16 in little-endian byte order.
    public mutating func writeU16(_ value: UInt16) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes a UInt32 in little-endian byte order.
    public mutating func writeU32(_ value: UInt32) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes a UInt64 in little-endian byte order.
    public mutating func writeU64(_ value: UInt64) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes an Int16 in little-endian byte order.
    public mutating func writeI16(_ value: Int16) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes an Int32 in little-endian byte order.
    public mutating func writeI32(_ value: Int32) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes an Int64 in little-endian byte order.
    public mutating func writeI64(_ value: Int64) {
        withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) }
    }

    /// Writes a Float (IEEE 754 binary32) in little-endian byte order.
    public mutating func writeF32(_ value: Float) {
        writeU32(value.bitPattern)
    }

    /// Writes a Double (IEEE 754 binary64) in little-endian byte order.
    public mutating func writeF64(_ value: Double) {
        writeU64(value.bitPattern)
    }
}
