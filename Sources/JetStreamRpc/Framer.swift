// JetStream RPC — Framer Protocol
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.framer]

import Foundation
import JetStreamWireFormat

/// Protocol for message types that can be encoded/decoded with a type discriminator.
public protocol Framer: Sendable {
    /// The message type discriminator byte.
    func messageType() -> UInt8
    /// The encoded size in bytes (not including frame header).
    func byteSize() -> UInt32
    /// Encode the message payload.
    func encode(writer: inout BinaryWriter) throws
    /// Decode a message given the type byte.
    static func decode(reader: inout BinaryReader, type: UInt8) throws -> Self
}
