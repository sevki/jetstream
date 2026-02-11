/**
 * r[impl jetstream.react.webtransport.stream]
 */
import type { Frame, Framer, FramerDecode } from '@sevki/jetstream-rpc';
import type { Transport } from '@sevki/jetstream-rpc';
import { BinaryReader, BinaryWriter, u8Codec, u16Codec, u32Codec } from '@sevki/jetstream-wireformat';

/**
 * Transport implementation over a single WebTransport bidirectional stream.
 * Encodes outgoing request frames and decodes incoming response frames using
 * the standard [size:u32 LE][type:u8][tag:u16 LE][payload] wire format.
 */
export class WebTransportTransport<TReq extends Framer, TRes extends Framer>
  implements Transport<TReq, TRes>
{
  private writer: WritableStreamDefaultWriter<Uint8Array>;
  private readable: ReadableStream<Uint8Array>;
  private responseDecode: FramerDecode<TRes>;

  constructor(
    stream: { readable: ReadableStream<Uint8Array>; writable: WritableStream<Uint8Array> },
    responseDecode: FramerDecode<TRes>,
  ) {
    this.writer = stream.writable.getWriter();
    this.readable = stream.readable;
    this.responseDecode = responseDecode;
  }

  async send(frame: Frame<TReq>): Promise<void> {
    const payloadSize = frame.msg.byteSize();
    const totalSize = 4 + 1 + 2 + payloadSize;
    const writer = new BinaryWriter();
    u32Codec.encode(totalSize, writer);
    u8Codec.encode(frame.msg.messageType(), writer);
    u16Codec.encode(frame.tag, writer);
    frame.msg.encode(writer);
    await this.writer.write(writer.toUint8Array());
  }

  async *receive(): AsyncGenerator<Frame<TRes>> {
    const reader = this.readable.getReader();
    let buffer = new Uint8Array(0);

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        // Append to buffer
        const newBuf = new Uint8Array(buffer.length + value.length);
        newBuf.set(buffer);
        newBuf.set(value, buffer.length);
        buffer = newBuf;

        // Extract complete frames
        while (buffer.length >= 4) {
          const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
          const size = view.getUint32(0, true);
          if (size < 7) throw new Error(`frame size ${size} < 7`);
          if (buffer.length < size) break;

          const frameBytes = buffer.slice(0, size);
          buffer = buffer.slice(size);

          const binReader = new BinaryReader(frameBytes);
          u32Codec.decode(binReader); // consume size
          const type = u8Codec.decode(binReader);
          const tag = u16Codec.decode(binReader);
          const msg = this.responseDecode(binReader, type);
          yield { tag, msg };
        }
      }
    } finally {
      reader.releaseLock();
    }
  }

  async close(): Promise<void> {
    this.writer.releaseLock();
  }
}
