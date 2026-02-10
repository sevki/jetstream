// JetStream WireFormat — Collection Types
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// MARK: - Array (Vec)

// r[impl jetstream.wireformat.vec]
// r[impl jetstream.wireformat.swift.array]
extension Array: WireFormat where Element: WireFormat {
    public func byteSize() -> UInt32 {
        var size: UInt32 = 2 // u16 count prefix
        for element in self {
            size += element.byteSize()
        }
        return size
    }

    public func encode(writer: inout BinaryWriter) throws {
        guard self.count <= Int(UInt16.max) else {
            throw WireFormatError.tooManyElements(self.count)
        }
        try UInt16(self.count).encode(writer: &writer)
        for element in self {
            try element.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> [Element] {
        let count = try UInt16.decode(reader: &reader)
        var result: [Element] = []
        result.reserveCapacity(Int(count))
        for _ in 0..<count {
            result.append(try Element.decode(reader: &reader))
        }
        return result
    }
}

// MARK: - WireData (Data with u32 length prefix)

// r[impl jetstream.wireformat.data]
// r[impl jetstream.wireformat.swift.data]
/// A byte buffer encoded with a u32 length prefix (unlike `[UInt8]` which uses u16).
/// Maximum length is 32 MB (33,554,432 bytes).
public struct WireData: Equatable {
    public var bytes: Data

    public init(_ bytes: Data = Data()) {
        self.bytes = bytes
    }

    public init(_ bytes: [UInt8]) {
        self.bytes = Data(bytes)
    }
}

private let maxDataLength: UInt32 = 32 * 1024 * 1024

extension WireData: WireFormat {
    public func byteSize() -> UInt32 {
        4 + UInt32(bytes.count)
    }

    public func encode(writer: inout BinaryWriter) throws {
        try UInt32(bytes.count).encode(writer: &writer)
        writer.writeBytes(bytes)
    }

    public static func decode(reader: inout BinaryReader) throws -> WireData {
        let length = try UInt32.decode(reader: &reader)
        guard length <= maxDataLength else {
            throw WireFormatError.dataTooLarge(length)
        }
        guard reader.remaining >= Int(length) else {
            throw WireFormatError.unexpectedEOF
        }
        let bytes = try reader.readBytes(count: Int(length))
        return WireData(bytes)
    }
}

// MARK: - OrderedMap (BTreeMap equivalent)

// r[impl jetstream.wireformat.map]
// r[impl jetstream.wireformat.swift.dict]
/// An ordered map that encodes key-value pairs sorted by key.
/// Uses a sorted array of tuples internally.
public struct OrderedMap<K: WireFormat & Comparable, V: WireFormat>: Equatable
    where K: Equatable, V: Equatable
{
    public var entries: [(key: K, value: V)]

    public init() {
        self.entries = []
    }

    public init(_ entries: [(key: K, value: V)]) {
        self.entries = entries.sorted { $0.key < $1.key }
    }

    public static func == (lhs: OrderedMap, rhs: OrderedMap) -> Bool {
        guard lhs.entries.count == rhs.entries.count else { return false }
        for (l, r) in zip(lhs.entries, rhs.entries) {
            if l.key != r.key || l.value != r.value { return false }
        }
        return true
    }
}

extension OrderedMap: WireFormat {
    public func byteSize() -> UInt32 {
        var size: UInt32 = 2
        for (key, value) in entries {
            size += key.byteSize() + value.byteSize()
        }
        return size
    }

    public func encode(writer: inout BinaryWriter) throws {
        guard entries.count <= Int(UInt16.max) else {
            throw WireFormatError.tooManyElements(entries.count)
        }
        try UInt16(entries.count).encode(writer: &writer)
        let sorted = entries.sorted { $0.key < $1.key }
        for (key, value) in sorted {
            try key.encode(writer: &writer)
            try value.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> OrderedMap<K, V> {
        let count = try UInt16.decode(reader: &reader)
        var entries: [(key: K, value: V)] = []
        entries.reserveCapacity(Int(count))
        for _ in 0..<count {
            let key = try K.decode(reader: &reader)
            let value = try V.decode(reader: &reader)
            entries.append((key: key, value: value))
        }
        return OrderedMap(entries)
    }
}

// MARK: - OrderedSet (BTreeSet equivalent)

// r[impl jetstream.wireformat.set]
// r[impl jetstream.wireformat.swift.set]
/// An ordered set that encodes elements in sorted order.
public struct OrderedSet<T: WireFormat & Comparable & Hashable>: Equatable {
    public var elements: [T]

    public init() {
        self.elements = []
    }

    public init(_ elements: [T]) {
        self.elements = elements.sorted()
    }
}

extension OrderedSet: WireFormat {
    public func byteSize() -> UInt32 {
        var size: UInt32 = 2
        for element in elements {
            size += element.byteSize()
        }
        return size
    }

    public func encode(writer: inout BinaryWriter) throws {
        guard elements.count <= Int(UInt16.max) else {
            throw WireFormatError.tooManyElements(elements.count)
        }
        try UInt16(elements.count).encode(writer: &writer)
        let sorted = elements.sorted()
        for element in sorted {
            try element.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> OrderedSet<T> {
        let count = try UInt16.decode(reader: &reader)
        var elements: [T] = []
        elements.reserveCapacity(Int(count))
        for _ in 0..<count {
            elements.append(try T.decode(reader: &reader))
        }
        return OrderedSet(elements)
    }
}
