// JetStream WireFormat — Error Type Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

final class ErrorTests: XCTestCase {

    // MARK: - Level

    // r[verify jetstream.wireformat.swift.level]
    // r[verify jetstream.wireformat.level]
    func testLevelRoundTrip() throws {
        for level in [Level.trace, .debug, .info, .warn, .error] {
            var writer = BinaryWriter()
            try level.encode(writer: &writer)
            var reader = BinaryReader(data: writer.data)
            let decoded = try Level.decode(reader: &reader)
            XCTAssertEqual(decoded, level)
        }
    }

    func testLevelEncoding() throws {
        var writer = BinaryWriter()
        try Level.error.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([4]))
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testLevelInvalidValue() throws {
        var reader = BinaryReader(data: Data([5]))
        XCTAssertThrowsError(try Level.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidLevelValue(5))
        }
    }

    // MARK: - FieldPair

    // r[verify jetstream.wireformat.swift.fieldpair]
    // r[verify jetstream.wireformat.fieldpair]
    func testFieldPairRoundTrip() throws {
        let pair = FieldPair(key: 1, value: 2)
        var writer = BinaryWriter()
        try pair.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try FieldPair.decode(reader: &reader)
        XCTAssertEqual(decoded, pair)
    }

    func testFieldPairByteSize() {
        XCTAssertEqual(FieldPair(key: 0, value: 0).byteSize(), 4)
    }

    // MARK: - Frame

    // r[verify jetstream.wireformat.swift.frame]
    // r[verify jetstream.wireformat.frame]
    func testFrameRoundTrip() throws {
        let frame = Frame(
            msg: "test span",
            name: 1,
            target: 2,
            module: 3,
            file: 4,
            line: 42,
            fields: [FieldPair(key: 5, value: 6)],
            level: .info
        )
        var writer = BinaryWriter()
        try frame.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try Frame.decode(reader: &reader)
        XCTAssertEqual(decoded, frame)
    }

    func testFrameEmptyFields() throws {
        let frame = Frame(
            msg: "",
            name: 0,
            target: 0,
            module: 0,
            file: 0,
            line: 0,
            fields: [],
            level: .trace
        )
        var writer = BinaryWriter()
        try frame.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try Frame.decode(reader: &reader)
        XCTAssertEqual(decoded, frame)
    }

    // MARK: - Backtrace

    // r[verify jetstream.wireformat.swift.backtrace]
    // r[verify jetstream.wireformat.backtrace]
    func testBacktraceRoundTrip() throws {
        let bt = Backtrace(
            internTable: ["", "myFunc", "myModule", "src/main.swift"],
            frames: [
                Frame(
                    msg: "doing work",
                    name: 1,
                    target: 2,
                    module: 2,
                    file: 3,
                    line: 10,
                    fields: [],
                    level: .info
                ),
            ]
        )
        var writer = BinaryWriter()
        try bt.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try Backtrace.decode(reader: &reader)
        XCTAssertEqual(decoded, bt)
    }

    func testEmptyBacktrace() throws {
        let bt = Backtrace()
        var writer = BinaryWriter()
        try bt.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try Backtrace.decode(reader: &reader)
        XCTAssertEqual(decoded.internTable, [""])
        XCTAssertEqual(decoded.frames.count, 0)
    }

    // MARK: - ErrorInner

    // r[verify jetstream.wireformat.swift.error-inner]
    // r[verify jetstream.wireformat.error-inner]
    func testErrorInnerRoundTrip() throws {
        let inner = ErrorInner(
            message: "something went wrong",
            code: "E001",
            help: "try restarting",
            url: "https://example.com/help"
        )
        var writer = BinaryWriter()
        try inner.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try ErrorInner.decode(reader: &reader)
        XCTAssertEqual(decoded, inner)
    }

    func testErrorInnerMinimal() throws {
        let inner = ErrorInner(message: "error")
        var writer = BinaryWriter()
        try inner.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try ErrorInner.decode(reader: &reader)
        XCTAssertEqual(decoded.message, "error")
        XCTAssertNil(decoded.code)
        XCTAssertNil(decoded.help)
        XCTAssertNil(decoded.url)
    }

    // MARK: - JetStreamError

    // r[verify jetstream.wireformat.swift.error]
    // r[verify jetstream.wireformat.error]
    func testJetStreamErrorRoundTrip() throws {
        let err = JetStreamError(
            inner: ErrorInner(
                message: "connection failed",
                code: "CONN_ERR",
                help: nil,
                url: nil
            ),
            backtrace: Backtrace(
                internTable: ["", "connect", "network"],
                frames: [
                    Frame(
                        msg: "connecting",
                        name: 1,
                        target: 2,
                        module: 2,
                        file: 0,
                        line: 55,
                        fields: [],
                        level: .error
                    ),
                ]
            )
        )
        var writer = BinaryWriter()
        try err.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try JetStreamError.decode(reader: &reader)
        XCTAssertEqual(decoded, err)
    }

    func testJetStreamErrorMinimal() throws {
        let err = JetStreamError(inner: ErrorInner(message: "fail"))
        var writer = BinaryWriter()
        try err.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try JetStreamError.decode(reader: &reader)
        XCTAssertEqual(decoded.inner.message, "fail")
        XCTAssertEqual(decoded.backtrace.frames.count, 0)
    }

    func testJetStreamErrorIsSwiftError() {
        let err = JetStreamError(inner: ErrorInner(message: "test"))
        // Verify it conforms to Swift's Error protocol
        let _: Error = err
    }

    // r[verify jetstream.wireformat.swift.test-roundtrip]
    func testFullErrorByteSize() {
        let err = JetStreamError(
            inner: ErrorInner(message: "x"),
            backtrace: Backtrace()
        )
        // inner: "x" = 2+1=3, code=nil=1, help=nil=1, url=nil=1 => 6
        // backtrace: internTable=[""] => 2+2+0=4, frames=[] => 2 => 6
        // total = 6 + 6 = 12
        XCTAssertEqual(err.byteSize(), 12)
    }
}
