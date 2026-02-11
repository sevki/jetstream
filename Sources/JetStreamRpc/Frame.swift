// JetStream RPC — Frame
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.frame]

import Foundation
import JetStreamWireFormat

/// RPC frame error.
public enum FrameError: Error, Equatable {
    case frameTooSmall(UInt32)
    case unknownMessageType(UInt8)
}

/// Message envelope: [size:u32 LE][type:u8][tag:u16 LE][payload]
/// Size includes itself (minimum 7 bytes: 4 + 1 + 2).
public struct Frame<T: Framer>: WireFormat {
    public var tag: UInt16
    public var msg: T

    public init(tag: UInt16, msg: T) {
        self.tag = tag
        self.msg = msg
    }

    public func byteSize() -> UInt32 {
        // size(4) + type(1) + tag(2) + payload
        4 + 1 + 2 + msg.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        let totalSize = byteSize()
        writer.writeU32(totalSize)
        writer.writeU8(msg.messageType())
        writer.writeU16(tag)
        try msg.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Frame<T> {
        let size = try reader.readU32()
        guard size >= 4 else {
            throw FrameError.frameTooSmall(size)
        }
        let type = try reader.readU8()
        let tag = try reader.readU16()
        let msg = try T.decode(reader: &reader, type: type)
        return Frame(tag: tag, msg: msg)
    }
}
