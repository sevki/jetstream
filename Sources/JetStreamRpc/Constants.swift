// JetStream RPC — Constants
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.message-ids]
// r[impl jetstream.rpc.swift.error-frame]

/// Starting message ID for service methods.
public let MESSAGE_ID_START: UInt8 = 102

/// Error response type ID (TLERROR - 1 = 5).
public let RJETSTREAMERROR: UInt8 = 5

/// Version negotiation message type IDs.
public let TVERSION: UInt8 = 100
public let RVERSION: UInt8 = 101
