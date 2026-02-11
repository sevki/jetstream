// JetStream RPC — Protocol
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.protocol]

import Foundation
import JetStreamWireFormat

/// Defines the request and response types for the JetStream protocol.
public protocol JetStreamProtocol {
    associatedtype Request: Framer
    associatedtype Response: Framer
    associatedtype Error: Swift.Error & Sendable
    static var VERSION: String { get }
    static var NAME: String { get }
}
