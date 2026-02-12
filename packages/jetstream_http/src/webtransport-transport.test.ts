/**
 * r[verify jetstream.webtransport.stream]
 * r[verify jetstream.rpc.ts.transport]
 */
import { describe, test, expect } from "vitest";
import { WebTransportTransport } from "./webtransport-transport.js";
import type { Framer } from "@sevki/jetstream-rpc";
import type { Frame, FramerDecode } from "@sevki/jetstream-rpc";
import {
  BinaryReader,
  BinaryWriter,
  u32Codec,
  u8Codec,
  u16Codec,
} from "@sevki/jetstream-wireformat";

/** A simple Framer that carries a single u32 payload. */
class SimpleMsg implements Framer {
  constructor(
    public type: number,
    public value: number,
  ) {}

  messageType(): number {
    return this.type;
  }

  byteSize(): number {
    return 4;
  }

  encode(writer: BinaryWriter): void {
    u32Codec.encode(this.value, writer);
  }
}

const simpleDecode: FramerDecode<SimpleMsg> = (
  reader: BinaryReader,
  type: number,
): SimpleMsg => {
  const value = u32Codec.decode(reader);
  return new SimpleMsg(type, value);
};

/** Encode a frame into raw wire bytes. */
function encodeFrame(tag: number, msg: SimpleMsg): Uint8Array {
  const bw = new BinaryWriter();
  const totalSize = 4 + 1 + 2 + msg.byteSize();
  u32Codec.encode(totalSize, bw);
  u8Codec.encode(msg.messageType(), bw);
  u16Codec.encode(tag, bw);
  msg.encode(bw);
  return bw.toUint8Array();
}

/** Create a mock bidi stream from a TransformStream. */
function mockBidiStream(): {
  stream: {
    readable: ReadableStream<Uint8Array>;
    writable: WritableStream<Uint8Array>;
  };
  pushToReadable: (data: Uint8Array) => void;
  closeReadable: () => void;
  getWritten: () => Promise<Uint8Array[]>;
} {
  const readableController: {
    ctrl?: ReadableStreamDefaultController<Uint8Array>;
  } = {};
  const readable = new ReadableStream<Uint8Array>({
    start(controller) {
      readableController.ctrl = controller;
    },
  });

  const writtenChunks: Uint8Array[] = [];
  const writable = new WritableStream<Uint8Array>({
    write(chunk) {
      writtenChunks.push(chunk);
    },
  });

  return {
    stream: { readable, writable },
    pushToReadable: (data: Uint8Array) =>
      readableController.ctrl!.enqueue(data),
    closeReadable: () => readableController.ctrl!.close(),
    getWritten: async () => writtenChunks,
  };
}

describe("WebTransportTransport", () => {
  test("send encodes frame in correct wire format", async () => {
    const { stream, closeReadable, getWritten } = mockBidiStream();
    const transport = new WebTransportTransport<SimpleMsg, SimpleMsg>(
      stream,
      simpleDecode,
    );

    await transport.send({ tag: 42, msg: new SimpleMsg(10, 0xdeadbeef) });

    const written = await getWritten();
    expect(written.length).toBe(1);

    const bytes = written[0];
    const br = new BinaryReader(bytes);
    const size = u32Codec.decode(br);
    expect(size).toBe(11); // 4 + 1 + 2 + 4
    const type = u8Codec.decode(br);
    expect(type).toBe(10);
    const tag = u16Codec.decode(br);
    expect(tag).toBe(42);
    const value = u32Codec.decode(br);
    expect(value).toBe(0xdeadbeef);

    closeReadable();
    await transport.close();
  });

  test("receive decodes incoming frames", async () => {
    const { stream, pushToReadable, closeReadable } = mockBidiStream();
    const transport = new WebTransportTransport<SimpleMsg, SimpleMsg>(
      stream,
      simpleDecode,
    );

    // Push two frames
    const frame1 = encodeFrame(1, new SimpleMsg(10, 100));
    const frame2 = encodeFrame(2, new SimpleMsg(20, 200));
    pushToReadable(frame1);
    pushToReadable(frame2);
    closeReadable();

    const received: Frame<SimpleMsg>[] = [];
    for await (const frame of transport.receive()) {
      received.push(frame);
    }

    expect(received.length).toBe(2);
    expect(received[0].tag).toBe(1);
    expect(received[0].msg.type).toBe(10);
    expect(received[0].msg.value).toBe(100);
    expect(received[1].tag).toBe(2);
    expect(received[1].msg.type).toBe(20);
    expect(received[1].msg.value).toBe(200);

    await transport.close();
  });

  test("receive handles fragmented frames", async () => {
    const { stream, pushToReadable, closeReadable } = mockBidiStream();
    const transport = new WebTransportTransport<SimpleMsg, SimpleMsg>(
      stream,
      simpleDecode,
    );

    // Encode a frame and split it into two chunks
    const frameBytes = encodeFrame(5, new SimpleMsg(42, 0xcafe));
    const mid = Math.floor(frameBytes.length / 2);
    pushToReadable(frameBytes.slice(0, mid));
    pushToReadable(frameBytes.slice(mid));
    closeReadable();

    const received: Frame<SimpleMsg>[] = [];
    for await (const frame of transport.receive()) {
      received.push(frame);
    }

    expect(received.length).toBe(1);
    expect(received[0].tag).toBe(5);
    expect(received[0].msg.type).toBe(42);
    expect(received[0].msg.value).toBe(0xcafe);

    await transport.close();
  });

  test("receive handles multiple frames in single chunk", async () => {
    const { stream, pushToReadable, closeReadable } = mockBidiStream();
    const transport = new WebTransportTransport<SimpleMsg, SimpleMsg>(
      stream,
      simpleDecode,
    );

    // Concatenate two frames into a single chunk
    const frame1 = encodeFrame(1, new SimpleMsg(10, 111));
    const frame2 = encodeFrame(2, new SimpleMsg(20, 222));
    const combined = new Uint8Array(frame1.length + frame2.length);
    combined.set(frame1);
    combined.set(frame2, frame1.length);
    pushToReadable(combined);
    closeReadable();

    const received: Frame<SimpleMsg>[] = [];
    for await (const frame of transport.receive()) {
      received.push(frame);
    }

    expect(received.length).toBe(2);
    expect(received[0].msg.value).toBe(111);
    expect(received[1].msg.value).toBe(222);

    await transport.close();
  });

  test("receive yields nothing on empty stream", async () => {
    const { stream, closeReadable } = mockBidiStream();
    const transport = new WebTransportTransport<SimpleMsg, SimpleMsg>(
      stream,
      simpleDecode,
    );

    closeReadable();

    const received: Frame<SimpleMsg>[] = [];
    for await (const frame of transport.receive()) {
      received.push(frame);
    }

    expect(received.length).toBe(0);
    await transport.close();
  });
});
