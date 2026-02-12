/**
 * r[impl jetstream.rpc.ts.server-codec]
 */
import { BinaryReader, BinaryWriter } from '@sevki/jetstream-wireformat';
import { u8Codec, u16Codec, u32Codec } from '@sevki/jetstream-wireformat';

/**
 * External framer codec — matches the shape of generated
 * `tmessageFramer` / `rmessageFramer` objects.
 */
export interface FramerCodec<T> {
  messageType(msg: T): number;
  byteSize(msg: T): number;
  encode(msg: T, writer: BinaryWriter): void;
  decode(reader: BinaryReader, type: number): T;
}

/**
 * ServerCodec decodes incoming bytes into request frames and encodes
 * outgoing response frames. It reads the 4-byte LE size prefix to
 * determine frame boundaries, then decodes the full frame.
 */
export class ServerCodec<TReq, TRes> {
  private buffer = new Uint8Array(0);

  constructor(
    private requestCodec: FramerCodec<TReq>,
    private responseCodec: FramerCodec<TRes>,
  ) {}

  /**
   * Read request frames from a ReadableStream, yielding each decoded frame.
   */
  async *decodeRequests(
    readable: ReadableStream<Uint8Array>,
  ): AsyncGenerator<{ tag: number; msg: TReq }> {
    const reader = readable.getReader();
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (value) {
          yield* this.feed(value);
        }
      }
    } finally {
      reader.releaseLock();
    }
  }

  /**
   * Feed a chunk of bytes and yield any complete frames.
   */
  *feed(chunk: Uint8Array): Generator<{ tag: number; msg: TReq }> {
    // Append chunk to internal buffer
    const newBuf = new Uint8Array(this.buffer.length + chunk.length);
    newBuf.set(this.buffer);
    newBuf.set(chunk, this.buffer.length);
    this.buffer = newBuf;

    // Extract complete frames
    while (this.buffer.length >= 4) {
      const view = new DataView(
        this.buffer.buffer,
        this.buffer.byteOffset,
        this.buffer.byteLength,
      );
      const size = view.getUint32(0, true); // LE
      if (size < 7) throw new Error(`frame size ${size} < 7`);
      if (this.buffer.length < size) break; // incomplete frame

      const frameBytes = this.buffer.slice(0, size);
      this.buffer = this.buffer.slice(size);

      const binReader = new BinaryReader(frameBytes);
      u32Codec.decode(binReader); // consume size field
      const type = u8Codec.decode(binReader);
      const tag = u16Codec.decode(binReader);
      const msg = this.requestCodec.decode(binReader, type);
      yield { tag, msg };
    }
  }

  /**
   * Encode a response frame to bytes.
   */
  encodeResponse(frame: { tag: number; msg: TRes }): Uint8Array {
    const payloadSize = this.responseCodec.byteSize(frame.msg);
    const totalSize = 4 + 1 + 2 + payloadSize;
    const writer = new BinaryWriter();
    u32Codec.encode(totalSize, writer);
    u8Codec.encode(this.responseCodec.messageType(frame.msg), writer);
    u16Codec.encode(frame.tag, writer);
    this.responseCodec.encode(frame.msg, writer);
    return writer.toUint8Array();
  }
}
