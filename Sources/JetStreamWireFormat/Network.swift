// JetStream WireFormat — Network Types
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// MARK: - IPv4Address

// r[impl jetstream.wireformat.ipv4]
// r[impl jetstream.wireformat.swift.ipv4]
/// An IPv4 address represented as 4 octets.
public struct IPv4Address: Equatable, Hashable {
    public var bytes: Data

    public init(_ a: UInt8, _ b: UInt8, _ c: UInt8, _ d: UInt8) {
        self.bytes = Data([a, b, c, d])
    }

    public init(octets: (UInt8, UInt8, UInt8, UInt8)) {
        self.bytes = Data([octets.0, octets.1, octets.2, octets.3])
    }

    /// Creates from a string like "192.168.1.1".
    public init?(string: String) {
        let parts = string.split(separator: ".")
        guard parts.count == 4,
              let a = UInt8(parts[0]),
              let b = UInt8(parts[1]),
              let c = UInt8(parts[2]),
              let d = UInt8(parts[3])
        else { return nil }
        self.bytes = Data([a, b, c, d])
    }

    public var description: String {
        "\(bytes[0]).\(bytes[1]).\(bytes[2]).\(bytes[3])"
    }
}

extension IPv4Address: WireFormat {
    public func byteSize() -> UInt32 { 4 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeBytes(bytes)
    }

    public static func decode(reader: inout BinaryReader) throws -> IPv4Address {
        let data = try reader.readBytes(count: 4)
        return IPv4Address(data[data.startIndex], data[data.startIndex + 1], data[data.startIndex + 2], data[data.startIndex + 3])
    }
}

// MARK: - IPv6Address

// r[impl jetstream.wireformat.ipv6]
// r[impl jetstream.wireformat.swift.ipv6]
/// An IPv6 address represented as 16 octets.
public struct IPv6Address: Equatable, Hashable {
    public var octets: Data

    public init(octets: Data) {
        precondition(octets.count == 16)
        self.octets = octets
    }

    public init(bytes: [UInt8]) {
        precondition(bytes.count == 16)
        self.octets = Data(bytes)
    }

    /// Creates the loopback address (::1).
    public static var loopback: IPv6Address {
        var bytes = [UInt8](repeating: 0, count: 16)
        bytes[15] = 1
        return IPv6Address(bytes: bytes)
    }
}

extension IPv6Address: WireFormat {
    public func byteSize() -> UInt32 { 16 }

    public func encode(writer: inout BinaryWriter) throws {
        writer.writeBytes(octets)
    }

    public static func decode(reader: inout BinaryReader) throws -> IPv6Address {
        let bytes = try reader.readBytes(count: 16)
        return IPv6Address(octets: bytes)
    }
}

// MARK: - IpAddr

// r[impl jetstream.wireformat.ipaddr]
// r[impl jetstream.wireformat.swift.ipaddr]
/// An IP address (either v4 or v6) with a type tag.
/// Tag 4 = IPv4, Tag 6 = IPv6.
public enum IpAddr: Equatable, Hashable {
    case v4(IPv4Address)
    case v6(IPv6Address)
}

extension IpAddr: WireFormat {
    public func byteSize() -> UInt32 {
        switch self {
        case .v4(let addr): return 1 + addr.byteSize()
        case .v6(let addr): return 1 + addr.byteSize()
        }
    }

    public func encode(writer: inout BinaryWriter) throws {
        switch self {
        case .v4(let addr):
            writer.writeU8(4)
            try addr.encode(writer: &writer)
        case .v6(let addr):
            writer.writeU8(6)
            try addr.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> IpAddr {
        let tag = try reader.readU8()
        switch tag {
        case 4: return .v4(try IPv4Address.decode(reader: &reader))
        case 6: return .v6(try IPv6Address.decode(reader: &reader))
        default: throw WireFormatError.invalidIPAddressTag(tag)
        }
    }
}

// MARK: - SocketAddr

// r[impl jetstream.wireformat.sockaddr-v4]
// r[impl jetstream.wireformat.sockaddr-v6]
// r[impl jetstream.wireformat.sockaddr]
// r[impl jetstream.wireformat.swift.sockaddr]
/// A socket address (IP + port) with a type tag.
/// Tag 4 = V4, Tag 6 = V6.
public enum SocketAddr: Equatable, Hashable {
    case v4(ip: IPv4Address, port: UInt16)
    case v6(ip: IPv6Address, port: UInt16)
}

extension SocketAddr: WireFormat {
    public func byteSize() -> UInt32 {
        switch self {
        case .v4(let ip, _): return 1 + ip.byteSize() + 2
        case .v6(let ip, _): return 1 + ip.byteSize() + 2
        }
    }

    public func encode(writer: inout BinaryWriter) throws {
        switch self {
        case .v4(let ip, let port):
            writer.writeU8(4)
            try ip.encode(writer: &writer)
            try port.encode(writer: &writer)
        case .v6(let ip, let port):
            writer.writeU8(6)
            try ip.encode(writer: &writer)
            try port.encode(writer: &writer)
        }
    }

    public static func decode(reader: inout BinaryReader) throws -> SocketAddr {
        let tag = try reader.readU8()
        switch tag {
        case 4:
            let ip = try IPv4Address.decode(reader: &reader)
            let port = try UInt16.decode(reader: &reader)
            return .v4(ip: ip, port: port)
        case 6:
            let ip = try IPv6Address.decode(reader: &reader)
            let port = try UInt16.decode(reader: &reader)
            return .v6(ip: ip, port: port)
        default:
            throw WireFormatError.invalidSocketAddrTag(tag)
        }
    }
}
