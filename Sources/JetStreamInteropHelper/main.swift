// r[impl jetstream.interop.swift]
// Swift interop helper: reads frames from stdin, decodes, re-encodes, writes to stdout.
// Protocol: [u8 type_tag][u32 LE payload_length][payload bytes], 0xFF = end sentinel.

import Foundation
import JetStreamWireFormat

let TAG_POINT: UInt8 = 1
let TAG_SHAPE: UInt8 = 2
let TAG_MESSAGE: UInt8 = 3
let TAG_END: UInt8 = 0xFF

/// Read exactly `count` bytes from a FileHandle.
func readExact(from handle: FileHandle, count: Int) throws -> Data {
    var result = Data()
    while result.count < count {
        let remaining = count - result.count
        let chunk = handle.readData(ofLength: remaining)
        if chunk.isEmpty {
            throw WireFormatError.unexpectedEOF
        }
        result.append(chunk)
    }
    return result
}

/// Decode and re-encode a value of a given WireFormat type from a payload.
func roundtrip<T: WireFormat>(_ type: T.Type, payload: Data) throws -> Data {
    var reader = BinaryReader(data: payload)
    let value = try T.decode(reader: &reader)
    var writer = BinaryWriter(capacity: Int(value.byteSize()))
    try value.encode(writer: &writer)
    return writer.data
}

let stdinHandle = FileHandle.standardInput
let stdoutHandle = FileHandle.standardOutput

while true {
    // Read type tag (1 byte)
    let tagData = try readExact(from: stdinHandle, count: 1)
    let tag = tagData[tagData.startIndex]

    if tag == TAG_END {
        // Echo back the end sentinel
        stdoutHandle.write(Data([TAG_END]))
        break
    }

    // Read payload length (u32 LE, 4 bytes)
    let lenData = try readExact(from: stdinHandle, count: 4)
    let len: UInt32 = lenData.withUnsafeBytes { $0.loadUnaligned(as: UInt32.self).littleEndian }

    // Read payload
    let payload = try readExact(from: stdinHandle, count: Int(len))

    // Decode then re-encode based on type tag
    let reEncoded: Data
    switch tag {
    case TAG_POINT:
        reEncoded = try roundtrip(Point.self, payload: payload)
    case TAG_SHAPE:
        reEncoded = try roundtrip(Shape.self, payload: payload)
    case TAG_MESSAGE:
        reEncoded = try roundtrip(Message.self, payload: payload)
    default:
        fputs("unknown type tag: \(tag)\n", stderr)
        exit(1)
    }

    // Write response: [tag][length LE][payload]
    var header = Data(count: 5)
    header[0] = tag
    let reLen = UInt32(reEncoded.count)
    withUnsafeBytes(of: reLen.littleEndian) { bytes in
        header.replaceSubrange(1..<5, with: bytes)
    }
    stdoutHandle.write(header)
    stdoutHandle.write(reEncoded)
}
