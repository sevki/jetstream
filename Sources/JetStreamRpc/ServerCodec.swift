// JetStream RPC — Server Codec
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

// r[impl jetstream.rpc.swift.server-codec]

import Foundation
import JetStreamWireFormat

/// Decodes incoming bytes into `Frame<P.Request>` and encodes outgoing
/// `Frame<P.Response>` for writing. Reads the 4-byte `size` prefix to
/// determine frame boundaries, then decodes the full frame.
public struct ServerCodec<P: JetStreamProtocol>: Sendable {
    public init() {}

    /// Decode request frames from an `AsyncSequence` of raw `Data` chunks.
    ///
    /// Accumulates incoming data and yields complete `Frame<P.Request>` values
    /// as soon as enough bytes have arrived (determined by the 4-byte LE size
    /// prefix).
    public func decodeFrames<S: AsyncSequence>(
        from source: S
    ) -> AsyncThrowingStream<Frame<P.Request>, Error> where S.Element == Data {
        AsyncThrowingStream { continuation in
            let task = Task {
                var buffer = Data()
                do {
                    for try await chunk in source {
                        buffer.append(chunk)
                        // Drain as many complete frames as possible.
                        while buffer.count >= 4 {
                            let size: UInt32 = buffer.withUnsafeBytes {
                                $0.loadUnaligned(as: UInt32.self)
                            }
                            let frameSize = Int(UInt32(littleEndian: size))
                            guard frameSize >= 7 else {
                                continuation.finish(throwing: FrameError.frameTooSmall(UInt32(frameSize)))
                                return
                            }
                            guard buffer.count >= frameSize else { break }
                            let frameData = buffer.prefix(frameSize)
                            var reader = BinaryReader(data: frameData)
                            let frame = try Frame<P.Request>.decode(reader: &reader)
                            continuation.yield(frame)
                            buffer = Data(buffer.dropFirst(frameSize))
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    /// Encode a response frame into raw bytes suitable for writing.
    public func encode(_ frame: Frame<P.Response>) throws -> Data {
        var writer = BinaryWriter(capacity: Int(frame.byteSize()))
        try frame.encode(writer: &writer)
        return writer.data
    }
}
