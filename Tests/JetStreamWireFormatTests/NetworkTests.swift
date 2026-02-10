// JetStream WireFormat — Network Type Tests
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import XCTest
@testable import JetStreamWireFormat

final class NetworkTests: XCTestCase {

    // MARK: - IPv4

    // r[verify jetstream.wireformat.swift.ipv4]
    // r[verify jetstream.wireformat.ipv4]
    func testIPv4RoundTrip() throws {
        let addr = IPv4Address(192, 168, 1, 1)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try IPv4Address.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testIPv4Encoding() throws {
        let addr = IPv4Address(10, 0, 0, 1)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([10, 0, 0, 1]))
    }

    func testIPv4ByteSize() {
        XCTAssertEqual(IPv4Address(0, 0, 0, 0).byteSize(), 4)
    }

    // MARK: - IPv6

    // r[verify jetstream.wireformat.swift.ipv6]
    // r[verify jetstream.wireformat.ipv6]
    func testIPv6RoundTrip() throws {
        let addr = IPv6Address.loopback
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try IPv6Address.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testIPv6Encoding() throws {
        let addr = IPv6Address.loopback
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var expected = Data(repeating: 0, count: 15)
        expected.append(1)
        XCTAssertEqual(writer.data, expected)
    }

    func testIPv6ByteSize() {
        XCTAssertEqual(IPv6Address.loopback.byteSize(), 16)
    }

    // MARK: - IpAddr

    // r[verify jetstream.wireformat.swift.ipaddr]
    // r[verify jetstream.wireformat.ipaddr]
    func testIpAddrV4RoundTrip() throws {
        let addr = IpAddr.v4(IPv4Address(127, 0, 0, 1))
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try IpAddr.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testIpAddrV6RoundTrip() throws {
        let addr = IpAddr.v6(IPv6Address.loopback)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try IpAddr.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testIpAddrV4Encoding() throws {
        let addr = IpAddr.v4(IPv4Address(10, 0, 0, 1))
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        XCTAssertEqual(writer.data, Data([0x04, 10, 0, 0, 1]))
    }

    func testIpAddrV6Tag() throws {
        let addr = IpAddr.v6(IPv6Address.loopback)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        XCTAssertEqual([UInt8](writer.data)[0], 0x06)
    }

    // r[verify jetstream.wireformat.swift.test-error]
    func testIpAddrInvalidTag() throws {
        var reader = BinaryReader(data: Data([0x05]))
        XCTAssertThrowsError(try IpAddr.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidIPAddressTag(5))
        }
    }

    func testIpAddrByteSize() {
        let v4 = IpAddr.v4(IPv4Address(0, 0, 0, 0))
        XCTAssertEqual(v4.byteSize(), 5) // 1 tag + 4

        let v6 = IpAddr.v6(IPv6Address.loopback)
        XCTAssertEqual(v6.byteSize(), 17) // 1 tag + 16
    }

    // MARK: - SocketAddr

    // r[verify jetstream.wireformat.swift.sockaddr]
    // r[verify jetstream.wireformat.sockaddr]
    // r[verify jetstream.wireformat.sockaddr-v4]
    // r[verify jetstream.wireformat.sockaddr-v6]
    func testSocketAddrV4RoundTrip() throws {
        let addr = SocketAddr.v4(ip: IPv4Address(192, 168, 0, 1), port: 8080)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try SocketAddr.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testSocketAddrV6RoundTrip() throws {
        let addr = SocketAddr.v6(ip: IPv6Address.loopback, port: 443)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        var reader = BinaryReader(data: writer.data)
        let decoded = try SocketAddr.decode(reader: &reader)
        XCTAssertEqual(decoded, addr)
    }

    func testSocketAddrV4Encoding() throws {
        let addr = SocketAddr.v4(ip: IPv4Address(127, 0, 0, 1), port: 80)
        var writer = BinaryWriter()
        try addr.encode(writer: &writer)
        // tag=4, ip=127.0.0.1, port=80 (0x50, 0x00 LE)
        XCTAssertEqual(writer.data, Data([0x04, 127, 0, 0, 1, 0x50, 0x00]))
    }

    func testSocketAddrByteSize() {
        let v4 = SocketAddr.v4(ip: IPv4Address(0, 0, 0, 0), port: 0)
        XCTAssertEqual(v4.byteSize(), 7) // 1 + 4 + 2

        let v6 = SocketAddr.v6(ip: IPv6Address.loopback, port: 0)
        XCTAssertEqual(v6.byteSize(), 19) // 1 + 16 + 2
    }

    func testSocketAddrInvalidTag() throws {
        var reader = BinaryReader(data: Data([0x05]))
        XCTAssertThrowsError(try SocketAddr.decode(reader: &reader)) { error in
            XCTAssertEqual(error as? WireFormatError, WireFormatError.invalidSocketAddrTag(5))
        }
    }
}
