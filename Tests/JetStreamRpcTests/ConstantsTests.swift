// JetStream RPC — Constants Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[verify jetstream.rpc.swift.message-ids]
// r[verify jetstream.rpc.swift.error-frame]

import XCTest
@testable import JetStreamRpc

final class ConstantsTests: XCTestCase {

    func testMessageIdStart() {
        XCTAssertEqual(MESSAGE_ID_START, 102)
    }

    func testRJetStreamError() {
        XCTAssertEqual(RJETSTREAMERROR, 5)
    }
}
