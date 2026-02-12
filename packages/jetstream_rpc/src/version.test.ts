/**
 * r[verify jetstream.version.negotiation.tversion]
 * r[verify jetstream.version.negotiation.rversion]
 * r[verify jetstream.version.framer.client-handshake]
 * r[verify jetstream.version.routing.identifiers]
 */
import { describe, test, expect } from "vitest";
import {
  negotiateVersion,
  acceptVersion,
  extractProtocolName,
} from "./version.js";
import type { NegotiatedVersion, AcceptedVersion } from "./version.js";
import { TVERSION, RVERSION } from "./constants.js";
import {
  BinaryReader,
  BinaryWriter,
  u8Codec,
  u16Codec,
  u32Codec,
  stringCodec,
} from "@sevki/jetstream-wireformat";

/** Create a ReadableStream from raw bytes. */
function readableFrom(bytes: Uint8Array): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(bytes);
      controller.close();
    },
  });
}

/** Create a WritableStream that collects all written bytes. */
function collectableWritable(): {
  writable: WritableStream<Uint8Array>;
  getBytes: () => Uint8Array;
} {
  const chunks: Uint8Array[] = [];
  const writable = new WritableStream<Uint8Array>({
    write(chunk) {
      chunks.push(chunk);
    },
  });
  return {
    writable,
    getBytes: () => {
      const total = chunks.reduce((sum, c) => sum + c.length, 0);
      const result = new Uint8Array(total);
      let offset = 0;
      for (const chunk of chunks) {
        result.set(chunk, offset);
        offset += chunk.length;
      }
      return result;
    },
  };
}

/** Encode a Tversion frame as raw bytes. */
function encodeTversionFrame(msize: number, version: string): Uint8Array {
  const bw = new BinaryWriter();
  // Payload: msize (u32) + version (string)
  const payloadBw = new BinaryWriter();
  u32Codec.encode(msize, payloadBw);
  stringCodec.encode(version, payloadBw);
  const payload = payloadBw.toUint8Array();

  const totalSize = 4 + 1 + 2 + payload.length;
  u32Codec.encode(totalSize, bw);
  u8Codec.encode(TVERSION, bw);
  u16Codec.encode(0xffff, bw); // NOTAG
  bw.writeBytes(payload);
  return bw.toUint8Array();
}

/** Encode an Rversion frame as raw bytes. */
function encodeRversionFrame(msize: number, version: string): Uint8Array {
  const bw = new BinaryWriter();
  const payloadBw = new BinaryWriter();
  u32Codec.encode(msize, payloadBw);
  stringCodec.encode(version, payloadBw);
  const payload = payloadBw.toUint8Array();

  const totalSize = 4 + 1 + 2 + payload.length;
  u32Codec.encode(totalSize, bw);
  u8Codec.encode(RVERSION, bw);
  u16Codec.encode(0xffff, bw);
  bw.writeBytes(payload);
  return bw.toUint8Array();
}

describe("extractProtocolName", () => {
  test("extracts name from full version string", () => {
    expect(
      extractProtocolName("rs.jetstream.proto/echohttp/15.0.0+bfd7d20e"),
    ).toBe("rs.jetstream.proto/echohttp");
  });

  test("returns full string if no slash", () => {
    expect(extractProtocolName("myprotocol")).toBe("myprotocol");
  });

  test("handles version string with only one slash", () => {
    expect(extractProtocolName("proto/1.0.0")).toBe("proto");
  });

  test("handles nested paths correctly", () => {
    expect(
      extractProtocolName("rs.jetstream.proto/deep/nested/1.0.0+abc"),
    ).toBe("rs.jetstream.proto/deep/nested");
  });
});

describe("negotiateVersion", () => {
  test("sends Tversion and receives Rversion", async () => {
    const protocolVersion = "rs.jetstream.proto/echo/1.0.0+abc123";
    const msize = 65536;

    // Server response: Rversion echoing the version back
    const rversionBytes = encodeRversionFrame(msize, protocolVersion);
    const readable = readableFrom(rversionBytes);
    const { writable, getBytes } = collectableWritable();

    const result = await negotiateVersion(
      readable,
      writable,
      protocolVersion,
      msize,
    );

    expect(result.msize).toBe(msize);
    expect(result.version).toBe(protocolVersion);

    // Verify the sent Tversion frame
    const sent = getBytes();
    const br = new BinaryReader(sent);
    const size = u32Codec.decode(br);
    expect(size).toBeGreaterThanOrEqual(7);
    const type = u8Codec.decode(br);
    expect(type).toBe(TVERSION);
    const tag = u16Codec.decode(br);
    expect(tag).toBe(0xffff); // NOTAG
    const sentMsize = u32Codec.decode(br);
    expect(sentMsize).toBe(msize);
    const sentVersion = stringCodec.decode(br);
    expect(sentVersion).toBe(protocolVersion);
  });

  test("throws when server rejects with 'unknown'", async () => {
    const rversionBytes = encodeRversionFrame(0, "unknown");
    const readable = readableFrom(rversionBytes);
    const { writable } = collectableWritable();

    await expect(
      negotiateVersion(readable, writable, "rs.jetstream.proto/test/1.0.0+abc"),
    ).rejects.toThrow("server rejected version negotiation");
  });

  test("throws when stream closes before Rversion", async () => {
    const readable = readableFrom(new Uint8Array(0));
    const { writable } = collectableWritable();

    await expect(
      negotiateVersion(readable, writable, "rs.jetstream.proto/test/1.0.0+abc"),
    ).rejects.toThrow("stream closed before Rversion received");
  });
});

describe("acceptVersion", () => {
  test("accepts known protocol and sends Rversion", async () => {
    const protocolVersion = "rs.jetstream.proto/echo/1.0.0+abc123";
    const tversionBytes = encodeTversionFrame(65536, protocolVersion);

    const readable = readableFrom(tversionBytes);
    const { writable, getBytes } = collectableWritable();

    const knownProtocols = new Set(["rs.jetstream.proto/echo"]);
    const result = await acceptVersion(readable, writable, knownProtocols);

    expect(result.protocolName).toBe("rs.jetstream.proto/echo");
    expect(result.version).toBe(protocolVersion);
    expect(result.msize).toBe(65536);

    // Verify Rversion was sent
    const sent = getBytes();
    const br = new BinaryReader(sent);
    u32Codec.decode(br); // size
    const type = u8Codec.decode(br);
    expect(type).toBe(RVERSION);
    u16Codec.decode(br); // tag
    const sentMsize = u32Codec.decode(br);
    expect(sentMsize).toBe(65536);
    const sentVersion = stringCodec.decode(br);
    expect(sentVersion).toBe(protocolVersion);
  });

  test("rejects unknown protocol with 'unknown' version", async () => {
    const tversionBytes = encodeTversionFrame(
      65536,
      "rs.jetstream.proto/unknown/1.0.0+abc",
    );

    const readable = readableFrom(tversionBytes);
    const { writable, getBytes } = collectableWritable();

    const knownProtocols = new Set(["rs.jetstream.proto/echo"]);
    await expect(
      acceptVersion(readable, writable, knownProtocols),
    ).rejects.toThrow("unknown protocol");

    // Verify Rversion with "unknown" was still sent
    const sent = getBytes();
    const br = new BinaryReader(sent);
    u32Codec.decode(br); // size
    u8Codec.decode(br); // type
    u16Codec.decode(br); // tag
    u32Codec.decode(br); // msize
    const sentVersion = stringCodec.decode(br);
    expect(sentVersion).toBe("unknown");
  });

  test("negotiates msize to the minimum", async () => {
    const clientMsize = 32768;
    const serverMsize = 16384;
    const tversionBytes = encodeTversionFrame(
      clientMsize,
      "rs.jetstream.proto/echo/1.0.0+abc",
    );

    const readable = readableFrom(tversionBytes);
    const { writable, getBytes } = collectableWritable();

    const knownProtocols = new Set(["rs.jetstream.proto/echo"]);
    const result = await acceptVersion(
      readable,
      writable,
      knownProtocols,
      serverMsize,
    );

    expect(result.msize).toBe(serverMsize); // min(32768, 16384)

    // Verify msize in Rversion
    const sent = getBytes();
    const br = new BinaryReader(sent);
    u32Codec.decode(br); // size
    u8Codec.decode(br); // type
    u16Codec.decode(br); // tag
    const sentMsize = u32Codec.decode(br);
    expect(sentMsize).toBe(serverMsize);
  });

  test("throws when stream closes before Tversion", async () => {
    const readable = readableFrom(new Uint8Array(0));
    const { writable } = collectableWritable();

    const knownProtocols = new Set(["rs.jetstream.proto/echo"]);
    await expect(
      acceptVersion(readable, writable, knownProtocols),
    ).rejects.toThrow("stream closed before Tversion received");
  });
});

describe("version handshake round-trip", () => {
  test("client and server negotiate successfully via piped streams", async () => {
    const protocolVersion = "rs.jetstream.proto/echo/2.0.0+deadbeef";

    // Create two pairs of streams: client→server and server→client
    const clientToServer = new TransformStream<Uint8Array, Uint8Array>();
    const serverToClient = new TransformStream<Uint8Array, Uint8Array>();

    const knownProtocols = new Set(["rs.jetstream.proto/echo"]);

    // Run client and server concurrently
    const [clientResult, serverResult] = await Promise.all([
      negotiateVersion(
        serverToClient.readable,
        clientToServer.writable,
        protocolVersion,
      ),
      acceptVersion(
        clientToServer.readable,
        serverToClient.writable,
        knownProtocols,
      ),
    ]);

    expect(clientResult.version).toBe(protocolVersion);
    expect(clientResult.msize).toBe(65536);

    expect(serverResult.protocolName).toBe("rs.jetstream.proto/echo");
    expect(serverResult.version).toBe(protocolVersion);
    expect(serverResult.msize).toBe(65536);
  });
});
