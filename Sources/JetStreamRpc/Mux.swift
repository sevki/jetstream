// JetStream RPC — Multiplexer
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.mux]

import Foundation
import JetStreamWireFormat

/// Client multiplexer for concurrent RPC calls.
public actor Mux<TReq: Framer, TRes: Framer> {
    private let tagPool: TagPool
    private var pending: [UInt16: CheckedContinuation<Frame<TRes>, Error>] = [:]
    private let transport: any Transport<TReq, TRes>

    public init(transport: any Transport<TReq, TRes>, maxConcurrent: UInt16 = 256) {
        self.tagPool = TagPool(maxConcurrent: maxConcurrent)
        self.transport = transport
    }

    /// Start the demux loop (call once).
    public func start() async {
        Task {
            for try await frame in transport.receive() {
                self.dispatch(frame)
            }
        }
    }

    private func dispatch(_ frame: Frame<TRes>) {
        if let continuation = pending.removeValue(forKey: frame.tag) {
            Task { await tagPool.release(frame.tag) }
            continuation.resume(returning: frame)
        }
    }

    /// Send a request and await the response.
    public func rpc(_ msg: TReq) async throws -> Frame<TRes> {
        guard let tag = await tagPool.acquire() else {
            throw FrameError.frameTooSmall(0) // no tags available
        }

        return try await withCheckedThrowingContinuation { continuation in
            pending[tag] = continuation
            Task {
                do {
                    try await transport.send(Frame(tag: tag, msg: msg))
                } catch {
                    if let cont = self.pending.removeValue(forKey: tag) {
                        Task { await self.tagPool.release(tag) }
                        cont.resume(throwing: error)
                    }
                }
            }
        }
    }

    public func close() async throws {
        try await transport.close()
    }
}
