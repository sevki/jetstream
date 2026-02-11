// JetStream RPC — Tag Pool
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.tag-pool]

import Foundation

/// Thread-safe tag pool for multiplexing.
public actor TagPool {
    private var available: [UInt16]

    public init(maxConcurrent: UInt16 = 256) {
        self.available = (1...maxConcurrent).reversed().map { $0 }
    }

    public func acquire() -> UInt16? {
        guard !available.isEmpty else { return nil }
        return available.removeLast()
    }

    public func release(_ tag: UInt16) {
        available.append(tag)
    }
}
