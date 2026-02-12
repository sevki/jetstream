/**
 * r[impl jetstream.version.negotiation.tversion]
 * r[impl jetstream.version.negotiation.rversion]
 * r[impl jetstream.version.framer.client-handshake]
 */
import {
  BinaryReader,
  BinaryWriter,
  u32Codec,
  u16Codec,
  u8Codec,
  stringCodec,
} from "@sevki/jetstream-wireformat";
import type { Framer } from "./frame.js";
import { TVERSION, RVERSION } from "./constants.js";

export interface Tversion extends Framer {
  msize: number;
  version: string;
}

export interface Rversion extends Framer {
  msize: number;
  version: string;
}

export function createTversion(msize: number, version: string): Tversion {
  return {
    msize,
    version,
    messageType() {
      return TVERSION;
    },
    byteSize() {
      return u32Codec.byteSize(msize) + stringCodec.byteSize(version);
    },
    encode(writer: BinaryWriter) {
      u32Codec.encode(msize, writer);
      stringCodec.encode(version, writer);
    },
  };
}

function decodeRversion(reader: BinaryReader): Rversion {
  const msize = u32Codec.decode(reader);
  const version = stringCodec.decode(reader);
  return {
    msize,
    version,
    messageType() {
      return RVERSION;
    },
    byteSize() {
      return u32Codec.byteSize(msize) + stringCodec.byteSize(version);
    },
    encode(writer: BinaryWriter) {
      u32Codec.encode(msize, writer);
      stringCodec.encode(version, writer);
    },
  };
}

export interface NegotiatedVersion {
  msize: number;
  version: string;
}

export interface AcceptedVersion {
  protocolName: string;
  msize: number;
  version: string;
}

function decodeTversion(reader: BinaryReader): Tversion {
  const msize = u32Codec.decode(reader);
  const version = stringCodec.decode(reader);
  return {
    msize,
    version,
    messageType() {
      return TVERSION;
    },
    byteSize() {
      return u32Codec.byteSize(msize) + stringCodec.byteSize(version);
    },
    encode(writer: BinaryWriter) {
      u32Codec.encode(msize, writer);
      stringCodec.encode(version, writer);
    },
  };
}

function createRversion(msize: number, version: string): Rversion {
  return {
    msize,
    version,
    messageType() {
      return RVERSION;
    },
    byteSize() {
      return u32Codec.byteSize(msize) + stringCodec.byteSize(version);
    },
    encode(writer: BinaryWriter) {
      u32Codec.encode(msize, writer);
      stringCodec.encode(version, writer);
    },
  };
}

/**
 * Extract protocol name from a version string like "rs.jetstream.proto/echohttp/15.0.0+bfd7d20e".
 * Returns the name portion "rs.jetstream.proto/echohttp", or the full string if no version suffix.
 */
export function extractProtocolName(versionString: string): string {
  const lastSlash = versionString.lastIndexOf("/");
  if (lastSlash === -1) return versionString;
  return versionString.substring(0, lastSlash);
}

/**
 * Server-side version negotiation: read Tversion from the stream,
 * look up a matching handler by protocol name, and send Rversion.
 * Returns the accepted version info including the protocol name for dispatch.
 */
export async function acceptVersion(
  readable: ReadableStream<Uint8Array>,
  writable: WritableStream<Uint8Array>,
  knownProtocols: Set<string>,
  msize: number = 65536,
): Promise<AcceptedVersion> {
  // Read the Tversion frame
  const reader = readable.getReader();
  let buffer = new Uint8Array(0);

  let tversion: Tversion;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) throw new Error("stream closed before Tversion received");

      const newBuf = new Uint8Array(buffer.length + value.length);
      newBuf.set(buffer);
      newBuf.set(value, buffer.length);
      buffer = newBuf;

      if (buffer.length < 4) continue;
      const view = new DataView(
        buffer.buffer,
        buffer.byteOffset,
        buffer.byteLength,
      );
      const size = view.getUint32(0, true);
      if (size < 7) throw new Error(`Tversion frame size ${size} < 7`);
      if (buffer.length < size) continue;

      const frameBytes = buffer.slice(0, size);
      const br = new BinaryReader(frameBytes);
      u32Codec.decode(br); // consume size
      const type = u8Codec.decode(br);
      u16Codec.decode(br); // consume tag

      if (type !== TVERSION) {
        throw new Error(
          `expected Tversion (type ${TVERSION}), got type ${type}`,
        );
      }

      tversion = decodeTversion(br);
      break;
    }
  } finally {
    reader.releaseLock();
  }

  const protocolName = extractProtocolName(tversion.version);
  const negotiatedMsize = Math.min(msize, tversion.msize);

  // Check if we know this protocol
  const accepted = knownProtocols.has(protocolName);
  const responseVersion = accepted ? tversion.version : "unknown";

  // Send Rversion
  const rversion = createRversion(negotiatedMsize, responseVersion);
  const payloadSize = rversion.byteSize();
  const totalSize = 4 + 1 + 2 + payloadSize;
  const bw = new BinaryWriter();
  u32Codec.encode(totalSize, bw);
  u8Codec.encode(RVERSION, bw);
  u16Codec.encode(0xffff, bw); // tag = NOTAG
  rversion.encode(bw);

  const writer = writable.getWriter();
  await writer.write(bw.toUint8Array());
  writer.releaseLock();

  if (!accepted) {
    throw new Error(`unknown protocol: ${protocolName}`);
  }

  return { protocolName, msize: negotiatedMsize, version: tversion.version };
}

/**
 * Perform Tversion/Rversion handshake on a raw bidirectional stream.
 * Sends a Tversion frame and waits for an Rversion response.
 * Returns the negotiated version, or throws if the server rejected.
 */
export async function negotiateVersion(
  readable: ReadableStream<Uint8Array>,
  writable: WritableStream<Uint8Array>,
  protocolVersion: string,
  msize: number = 65536,
): Promise<NegotiatedVersion> {
  const writer = writable.getWriter();

  // Encode and send Tversion frame: [size:u32][type:u8][tag:u16][msize:u32][version:string]
  const tversion = createTversion(msize, protocolVersion);
  const payloadSize = tversion.byteSize();
  const totalSize = 4 + 1 + 2 + payloadSize;
  const bw = new BinaryWriter();
  u32Codec.encode(totalSize, bw);
  u8Codec.encode(TVERSION, bw);
  u16Codec.encode(0xffff, bw); // tag = NOTAG for version messages
  tversion.encode(bw);
  await writer.write(bw.toUint8Array());
  writer.releaseLock();

  // Read Rversion response
  const reader = readable.getReader();
  let buffer = new Uint8Array(0);

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) throw new Error("stream closed before Rversion received");

      const newBuf = new Uint8Array(buffer.length + value.length);
      newBuf.set(buffer);
      newBuf.set(value, buffer.length);
      buffer = newBuf;

      if (buffer.length < 4) continue;
      const view = new DataView(
        buffer.buffer,
        buffer.byteOffset,
        buffer.byteLength,
      );
      const size = view.getUint32(0, true);
      if (size < 7) throw new Error(`Rversion frame size ${size} < 7`);
      if (buffer.length < size) continue;

      const frameBytes = buffer.slice(0, size);
      const br = new BinaryReader(frameBytes);
      u32Codec.decode(br); // consume size
      const type = u8Codec.decode(br);
      u16Codec.decode(br); // consume tag

      if (type !== RVERSION) {
        throw new Error(
          `expected Rversion (type ${RVERSION}), got type ${type}`,
        );
      }

      const rversion = decodeRversion(br);

      if (rversion.version === "unknown") {
        throw new Error("server rejected version negotiation");
      }

      return { msize: rversion.msize, version: rversion.version };
    }
  } finally {
    reader.releaseLock();
  }
}
