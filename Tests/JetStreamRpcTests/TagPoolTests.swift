// JetStream RPC — TagPool Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[verify jetstream.rpc.swift.tag-pool]

import XCTest
@testable import JetStreamRpc

final class TagPoolTests: XCTestCase {

    func testAcquireReturnsUniqueTags() async {
        let pool = TagPool(maxConcurrent: 3)
        let tag1 = await pool.acquire()
        let tag2 = await pool.acquire()
        let tag3 = await pool.acquire()

        XCTAssertNotNil(tag1)
        XCTAssertNotNil(tag2)
        XCTAssertNotNil(tag3)

        let tags = Set([tag1!, tag2!, tag3!])
        XCTAssertEqual(tags.count, 3, "All acquired tags should be unique")
    }

    func testAcquireReturnsNilWhenExhausted() async {
        let pool = TagPool(maxConcurrent: 2)
        _ = await pool.acquire()
        _ = await pool.acquire()
        let tag = await pool.acquire()
        XCTAssertNil(tag, "Should return nil when pool is exhausted")
    }

    func testReleaseMakesTagAvailable() async {
        let pool = TagPool(maxConcurrent: 1)
        let tag1 = await pool.acquire()
        XCTAssertNotNil(tag1)

        // Pool is now empty
        let exhausted = await pool.acquire()
        XCTAssertNil(exhausted)

        // Release the tag
        await pool.release(tag1!)

        // Now we can acquire again
        let tag2 = await pool.acquire()
        XCTAssertNotNil(tag2)
        XCTAssertEqual(tag1, tag2)
    }

    func testTagsStartFromOne() async {
        let pool = TagPool(maxConcurrent: 5)
        var tags: [UInt16] = []
        for _ in 0..<5 {
            if let tag = await pool.acquire() {
                tags.append(tag)
            }
        }
        // All tags should be in range 1...5
        for tag in tags {
            XCTAssertTrue(tag >= 1 && tag <= 5, "Tag \(tag) should be in range 1...5")
        }
    }
}
