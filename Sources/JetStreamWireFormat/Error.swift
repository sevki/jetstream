// JetStream WireFormat — Error Types
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// MARK: - Level

// r[impl jetstream.wireformat.level]
// r[impl jetstream.wireformat.swift.level]
/// Tracing level for backtrace frames.
public enum Level: UInt8, Equatable, Hashable {
    case trace = 0
    case debug = 1
    case info = 2
    case warn = 3
    case error = 4
}

extension Level: WireFormat {
    public func byteSize() -> UInt32 { 1 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeU8(self.rawValue)
    }

    public static func decode(reader: inout BinaryReader) throws -> Level {
        let byte = try reader.readU8()
        guard let level = Level(rawValue: byte) else {
            throw WireFormatError.invalidLevelValue(byte)
        }
        return level
    }
}

// MARK: - FieldPair

// r[impl jetstream.wireformat.fieldpair]
// r[impl jetstream.wireformat.swift.fieldpair]
/// A key-value pair referencing the intern table by u16 indices.
public struct FieldPair: Equatable, Hashable {
    public var key: UInt16
    public var value: UInt16

    public init(key: UInt16, value: UInt16) {
        self.key = key
        self.value = value
    }
}

extension FieldPair: WireFormat {
    public func byteSize() -> UInt32 { 4 }

    public func encode(writer: inout BinaryWriter) throws {
        try key.encode(writer: &writer)
        try value.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> FieldPair {
        let key = try UInt16.decode(reader: &reader)
        let value = try UInt16.decode(reader: &reader)
        return FieldPair(key: key, value: value)
    }
}

// MARK: - Frame

// r[impl jetstream.wireformat.frame]
// r[impl jetstream.wireformat.swift.frame]
/// A backtrace frame with intern table references.
public struct Frame: Equatable {
    public var msg: String
    public var name: UInt16
    public var target: UInt16
    public var module: UInt16
    public var file: UInt16
    public var line: UInt16
    public var fields: [FieldPair]
    public var level: Level

    public init(
        msg: String,
        name: UInt16,
        target: UInt16,
        module: UInt16,
        file: UInt16,
        line: UInt16,
        fields: [FieldPair],
        level: Level
    ) {
        self.msg = msg
        self.name = name
        self.target = target
        self.module = module
        self.file = file
        self.line = line
        self.fields = fields
        self.level = level
    }
}

extension Frame: WireFormat {
    public func byteSize() -> UInt32 {
        msg.byteSize()
            + name.byteSize()
            + target.byteSize()
            + module.byteSize()
            + file.byteSize()
            + line.byteSize()
            + fields.byteSize()
            + level.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try msg.encode(writer: &writer)
        try name.encode(writer: &writer)
        try target.encode(writer: &writer)
        try module.encode(writer: &writer)
        try file.encode(writer: &writer)
        try line.encode(writer: &writer)
        try fields.encode(writer: &writer)
        try level.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Frame {
        let msg = try String.decode(reader: &reader)
        let name = try UInt16.decode(reader: &reader)
        let target = try UInt16.decode(reader: &reader)
        let module = try UInt16.decode(reader: &reader)
        let file = try UInt16.decode(reader: &reader)
        let line = try UInt16.decode(reader: &reader)
        let fields = try [FieldPair].decode(reader: &reader)
        let level = try Level.decode(reader: &reader)
        return Frame(
            msg: msg,
            name: name,
            target: target,
            module: module,
            file: file,
            line: line,
            fields: fields,
            level: level
        )
    }
}

// MARK: - Backtrace

// r[impl jetstream.wireformat.backtrace]
// r[impl jetstream.wireformat.swift.backtrace]
/// A backtrace with an intern table for string deduplication.
/// Index 0 of the intern table is reserved for the empty string "".
public struct Backtrace: Equatable {
    public var internTable: [String]
    public var frames: [Frame]

    public init(internTable: [String] = [""], frames: [Frame] = []) {
        self.internTable = internTable
        self.frames = frames
    }
}

extension Backtrace: WireFormat {
    public func byteSize() -> UInt32 {
        internTable.byteSize() + frames.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try internTable.encode(writer: &writer)
        try frames.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Backtrace {
        let internTable = try [String].decode(reader: &reader)
        let frames = try [Frame].decode(reader: &reader)
        return Backtrace(internTable: internTable, frames: frames)
    }
}

// MARK: - ErrorInner

// r[impl jetstream.wireformat.error-inner]
// r[impl jetstream.wireformat.swift.error-inner]
/// The inner error struct containing message, code, help, and url fields.
public struct ErrorInner: Equatable {
    public var message: String
    public var code: String?
    public var help: String?
    public var url: String?

    public init(
        message: String,
        code: String? = nil,
        help: String? = nil,
        url: String? = nil
    ) {
        self.message = message
        self.code = code
        self.help = help
        self.url = url
    }
}

extension ErrorInner: WireFormat {
    public func byteSize() -> UInt32 {
        message.byteSize()
            + OptionalCoding.byteSize(code)
            + OptionalCoding.byteSize(help)
            + OptionalCoding.byteSize(url)
    }

    public func encode(writer: inout BinaryWriter) throws {
        try message.encode(writer: &writer)
        try OptionalCoding.encode(code, writer: &writer)
        try OptionalCoding.encode(help, writer: &writer)
        try OptionalCoding.encode(url, writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> ErrorInner {
        let message = try String.decode(reader: &reader)
        let code: String? = try OptionalCoding.decode(reader: &reader)
        let help: String? = try OptionalCoding.decode(reader: &reader)
        let url: String? = try OptionalCoding.decode(reader: &reader)
        return ErrorInner(message: message, code: code, help: help, url: url)
    }
}

// MARK: - JetStreamError

// r[impl jetstream.wireformat.error]
// r[impl jetstream.wireformat.swift.error]
/// The top-level JetStream error type with inner error and backtrace.
public struct JetStreamError: Equatable, Error {
    public var inner: ErrorInner
    public var backtrace: Backtrace

    public init(inner: ErrorInner, backtrace: Backtrace = Backtrace()) {
        self.inner = inner
        self.backtrace = backtrace
    }
}

extension JetStreamError: WireFormat {
    public func byteSize() -> UInt32 {
        inner.byteSize() + backtrace.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try inner.encode(writer: &writer)
        try backtrace.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> JetStreamError {
        let inner = try ErrorInner.decode(reader: &reader)
        let backtrace = try Backtrace.decode(reader: &reader)
        return JetStreamError(inner: inner, backtrace: backtrace)
    }
}
