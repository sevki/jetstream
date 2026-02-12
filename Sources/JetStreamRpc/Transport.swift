// JetStream RPC — Transport
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.transport]

import Foundation
import JetStreamWireFormat

/// Transport protocol for sending and receiving frames.
public protocol Transport<TReq, TRes>: Sendable where TReq: Framer, TRes: Framer {
    associatedtype TReq: Framer
    associatedtype TRes: Framer
    func send(_ frame: Frame<TReq>) async throws
    func receive() -> AsyncThrowingStream<Frame<TRes>, Error>
    func close() async throws
}
