// JetStream BinaryReader
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.swift.reader]
/// A reader that reads binary data from a `Data` buffer with a cursor position.
public struct BinaryReader {
    private let data: Data
    public private(set) var offset: Int

    /// Creates a new BinaryReader from the given data.
    public init(data: Data) {
        self.data = data
        self.offset = 0
    }

    /// The number of bytes remaining to be read.
    public var remaining: Int {
        return data.count - offset
    }

    /// Reads exactly `count` bytes from the buffer.
    public mutating func readBytes(count: Int) throws -> Data {
        guard offset + count <= data.count else {
            throw WireFormatError.unexpectedEOF
        }
        let result = data[data.startIndex + offset ..< data.startIndex + offset + count]
        offset += count
        return result
    }

    /// Reads a single UInt8.
    public mutating func readU8() throws -> UInt8 {
        guard offset < data.count else {
            throw WireFormatError.unexpectedEOF
        }
        let value = data[data.startIndex + offset]
        offset += 1
        return value
    }

    // r[impl jetstream.wireformat.byte-order]
    /// Reads a UInt16 in little-endian byte order.
    public mutating func readU16() throws -> UInt16 {
        let bytes = try readBytes(count: 2)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: UInt16.self).littleEndian }
    }

    /// Reads a UInt32 in little-endian byte order.
    public mutating func readU32() throws -> UInt32 {
        let bytes = try readBytes(count: 4)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: UInt32.self).littleEndian }
    }

    /// Reads a UInt64 in little-endian byte order.
    public mutating func readU64() throws -> UInt64 {
        let bytes = try readBytes(count: 8)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: UInt64.self).littleEndian }
    }

    /// Reads an Int16 in little-endian byte order.
    public mutating func readI16() throws -> Int16 {
        let bytes = try readBytes(count: 2)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: Int16.self).littleEndian }
    }

    /// Reads an Int32 in little-endian byte order.
    public mutating func readI32() throws -> Int32 {
        let bytes = try readBytes(count: 4)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: Int32.self).littleEndian }
    }

    /// Reads an Int64 in little-endian byte order.
    public mutating func readI64() throws -> Int64 {
        let bytes = try readBytes(count: 8)
        return bytes.withUnsafeBytes { $0.loadUnaligned(as: Int64.self).littleEndian }
    }

    /// Reads a Float (IEEE 754 binary32) in little-endian byte order.
    public mutating func readF32() throws -> Float {
        let bits = try readU32()
        return Float(bitPattern: bits)
    }

    /// Reads a Double (IEEE 754 binary64) in little-endian byte order.
    public mutating func readF64() throws -> Double {
        let bits = try readU64()
        return Double(bitPattern: bits)
    }
}
