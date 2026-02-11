// JetStream RPC — Version Negotiation
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.version.negotiation.tversion]
// r[impl jetstream.version.negotiation.rversion]
// r[impl jetstream.version.framer.client-handshake]

import Foundation
import JetStreamWireFormat

/// Tversion frame payload: msize + version string.
public struct Tversion: WireFormat {
    public var msize: UInt32
    public var version: String

    public init(msize: UInt32, version: String) {
        self.msize = msize
        self.version = version
    }

    public func byteSize() -> UInt32 {
        return msize.byteSize() + version.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try msize.encode(writer: &writer)
        try version.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Tversion {
        let msize = try UInt32.decode(reader: &reader)
        let version = try String.decode(reader: &reader)
        return Tversion(msize: msize, version: version)
    }
}

/// Rversion frame payload: msize + version string.
public struct Rversion: WireFormat {
    public var msize: UInt32
    public var version: String

    public init(msize: UInt32, version: String) {
        self.msize = msize
        self.version = version
    }

    public func byteSize() -> UInt32 {
        return msize.byteSize() + version.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try msize.encode(writer: &writer)
        try version.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Rversion {
        let msize = try UInt32.decode(reader: &reader)
        let version = try String.decode(reader: &reader)
        return Rversion(msize: msize, version: version)
    }
}

/// Result of a successful version negotiation.
public struct NegotiatedVersion: Sendable {
    public let msize: UInt32
    public let version: String

    public init(msize: UInt32, version: String) {
        self.msize = msize
        self.version = version
    }
}

/// Error during version negotiation.
public enum VersionNegotiationError: Error {
    case rejected
    case unexpectedMessageType(UInt8)
    case streamClosed
}
