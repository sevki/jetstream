/**
 * r[verify jetstream.rpc.ts.tag-pool]
 * r[verify jetstream.rpc.ts.frame]
 * r[verify jetstream.rpc.ts.framer]
 * r[verify jetstream.rpc.ts.mux]
 * r[verify jetstream.rpc.ts.protocol]
 */
import { describe, test, expect } from "vitest";
import {
  BinaryReader,
  BinaryWriter,
  u8Codec,
  u16Codec,
  u32Codec,
} from "@sevki/jetstream-wireformat";
import { TagPool } from "./tag-pool.js";
import { frameCodec } from "./frame.js";
import type { Framer, FramerDecode, Frame } from "./frame.js";
import type { Transport } from "./transport.js";
import { Mux } from "./mux.js";
import { MESSAGE_ID_START, RJETSTREAMERROR } from "./constants.js";
import type { Protocol } from "./protocol.js";

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
    return 4; // u32
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

describe("TagPool", () => {
  test("acquire returns tags starting from 1", () => {
    const pool = new TagPool(3);
    expect(pool.acquire()).toBe(1);
    expect(pool.acquire()).toBe(2);
    expect(pool.acquire()).toBe(3);
  });

  test("acquire returns null when exhausted", () => {
    const pool = new TagPool(1);
    expect(pool.acquire()).toBe(1);
    expect(pool.acquire()).toBeNull();
  });

  test("release makes tag available again", () => {
    const pool = new TagPool(1);
    const tag = pool.acquire()!;
    expect(pool.acquire()).toBeNull();
    pool.release(tag);
    expect(pool.acquire()).toBe(tag);
  });

  test("default pool has 256 tags", () => {
    const pool = new TagPool();
    const tags: number[] = [];
    for (let i = 0; i < 256; i++) {
      const t = pool.acquire();
      expect(t).not.toBeNull();
      tags.push(t!);
    }
    expect(pool.acquire()).toBeNull();
    // All tags are unique
    expect(new Set(tags).size).toBe(256);
  });
});

describe("frameCodec", () => {
  const codec = frameCodec(simpleDecode);

  test("encode/decode round-trip", () => {
    const frame: Frame<SimpleMsg> = {
      tag: 42,
      msg: new SimpleMsg(1, 0xdeadbeef),
    };
    const writer = new BinaryWriter();
    codec.encode(frame, writer);
    const bytes = writer.toUint8Array();

    const reader = new BinaryReader(bytes);
    const decoded = codec.decode(reader);

    expect(decoded.tag).toBe(42);
    expect(decoded.msg.type).toBe(1);
    expect(decoded.msg.value).toBe(0xdeadbeef);
  });

  test("byteSize is correct", () => {
    const frame: Frame<SimpleMsg> = { tag: 1, msg: new SimpleMsg(0, 0) };
    // 4 (size) + 1 (type) + 2 (tag) + 4 (payload) = 11
    expect(codec.byteSize(frame)).toBe(11);
  });

  test("wire format matches Rust layout", () => {
    const frame: Frame<SimpleMsg> = {
      tag: 0x0102,
      msg: new SimpleMsg(0xff, 0x04030201),
    };
    const writer = new BinaryWriter();
    codec.encode(frame, writer);
    const bytes = writer.toUint8Array();

    // Total size = 11 = 0x0B000000 LE
    expect(bytes[0]).toBe(0x0b);
    expect(bytes[1]).toBe(0x00);
    expect(bytes[2]).toBe(0x00);
    expect(bytes[3]).toBe(0x00);

    // type = 0xFF
    expect(bytes[4]).toBe(0xff);

    // tag = 0x0102 LE
    expect(bytes[5]).toBe(0x02);
    expect(bytes[6]).toBe(0x01);

    // payload = 0x04030201 LE
    expect(bytes[7]).toBe(0x01);
    expect(bytes[8]).toBe(0x02);
    expect(bytes[9]).toBe(0x03);
    expect(bytes[10]).toBe(0x04);
  });

  test("decode throws on frame size < 4", () => {
    const writer = new BinaryWriter();
    u32Codec.encode(3, writer); // size = 3, invalid
    const reader = new BinaryReader(writer.toUint8Array());
    expect(() => codec.decode(reader)).toThrow("frame size 3 < 4");
  });

  test("multiple frames in sequence", () => {
    const writer = new BinaryWriter();
    codec.encode({ tag: 1, msg: new SimpleMsg(10, 100) }, writer);
    codec.encode({ tag: 2, msg: new SimpleMsg(20, 200) }, writer);

    const reader = new BinaryReader(writer.toUint8Array());
    const f1 = codec.decode(reader);
    const f2 = codec.decode(reader);

    expect(f1.tag).toBe(1);
    expect(f1.msg.type).toBe(10);
    expect(f1.msg.value).toBe(100);

    expect(f2.tag).toBe(2);
    expect(f2.msg.type).toBe(20);
    expect(f2.msg.value).toBe(200);
  });
});

describe("Mux", () => {
  /** Create a mock transport that echoes frames back with type + 1. */
  function mockTransport(): Transport<SimpleMsg, SimpleMsg> & {
    sent: Frame<SimpleMsg>[];
    push(f: Frame<SimpleMsg>): void;
  } {
    const sent: Frame<SimpleMsg>[] = [];
    let resolveNext:
      | ((value: IteratorResult<Frame<SimpleMsg>>) => void)
      | null = null;
    const queue: Frame<SimpleMsg>[] = [];
    let done = false;

    return {
      sent,
      async send(frame: Frame<SimpleMsg>): Promise<void> {
        sent.push(frame);
        // Echo back with type + 1
        const response: Frame<SimpleMsg> = {
          tag: frame.tag,
          msg: new SimpleMsg(frame.msg.type + 1, frame.msg.value),
        };
        this.push(response);
      },
      push(f: Frame<SimpleMsg>) {
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r({ value: f, done: false });
        } else {
          queue.push(f);
        }
      },
      receive(): AsyncIterable<Frame<SimpleMsg>> {
        return {
          [Symbol.asyncIterator]() {
            return {
              next(): Promise<IteratorResult<Frame<SimpleMsg>>> {
                if (queue.length > 0) {
                  return Promise.resolve({
                    value: queue.shift()!,
                    done: false,
                  });
                }
                if (done) {
                  return Promise.resolve({
                    value: undefined as any,
                    done: true,
                  });
                }
                return new Promise((resolve) => {
                  resolveNext = resolve;
                });
              },
            };
          },
        };
      },
      async close(): Promise<void> {
        done = true;
        if (resolveNext) {
          resolveNext({ value: undefined as any, done: true });
        }
      },
    };
  }

  test("rpc sends and receives a response", async () => {
    const transport = mockTransport();
    const mux = new Mux<SimpleMsg, SimpleMsg>(transport, 16);
    await mux.start();

    const result = await mux.rpc(new SimpleMsg(1, 42));
    expect(result.msg.type).toBe(2); // echoed with type + 1
    expect(result.msg.value).toBe(42);

    await mux.close();
  });

  test("rpc throws when no tags available", async () => {
    const transport = mockTransport();
    const mux = new Mux<SimpleMsg, SimpleMsg>(transport, 1);
    await mux.start();

    // First rpc acquires the only tag — don't await it yet
    const first = mux.rpc(new SimpleMsg(1, 1));
    // The tag is consumed; second rpc should fail
    await expect(mux.rpc(new SimpleMsg(1, 2))).rejects.toThrow(
      "no tags available",
    );

    // Clean up: await first so it resolves
    await first;
    await mux.close();
  });

  test("concurrent rpcs get correct responses", async () => {
    const transport = mockTransport();
    const mux = new Mux<SimpleMsg, SimpleMsg>(transport, 16);
    await mux.start();

    const [r1, r2, r3] = await Promise.all([
      mux.rpc(new SimpleMsg(10, 100)),
      mux.rpc(new SimpleMsg(20, 200)),
      mux.rpc(new SimpleMsg(30, 300)),
    ]);

    expect(r1.msg.value).toBe(100);
    expect(r2.msg.value).toBe(200);
    expect(r3.msg.value).toBe(300);

    await mux.close();
  });
});

describe("constants", () => {
  test("MESSAGE_ID_START is 102", () => {
    expect(MESSAGE_ID_START).toBe(102);
  });

  test("RJETSTREAMERROR is 5", () => {
    expect(RJETSTREAMERROR).toBe(5);
  });
});

describe("Protocol", () => {
  test("can implement Protocol interface", () => {
    const proto: Protocol<SimpleMsg, SimpleMsg> = {
      VERSION: "1.0.0",
      NAME: "test",
    };
    expect(proto.VERSION).toBe("1.0.0");
    expect(proto.NAME).toBe("test");
  });
});
