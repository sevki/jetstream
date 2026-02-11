// JetStream RPC — Context
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.context]

import Foundation

/// RPC call context.
public struct Context: Sendable {
    public var remoteAddress: String?

    public init(remoteAddress: String? = nil) {
        self.remoteAddress = remoteAddress
    }

    public static let `default` = Context()
}
