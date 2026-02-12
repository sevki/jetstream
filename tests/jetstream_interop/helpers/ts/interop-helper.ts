// r[impl jetstream.interop.ts]
// TypeScript interop helper: reads frames from stdin, decodes, re-encodes, writes to stdout.

import fs from 'node:fs';
import { BinaryReader, BinaryWriter } from '@sevki/jetstream-wireformat';
import { pointCodec, shapeCodec, messageCodec } from './lib.js';

const TAG_POINT = 1;
const TAG_SHAPE = 2;
const TAG_MESSAGE = 3;
const TAG_END = 0xFF;

const stdin = fs.openSync('/dev/stdin', 'r');

function readExact(buf: Buffer, offset: number, length: number): void {
  let read = 0;
  while (read < length) {
    const n = fs.readSync(stdin, buf, offset + read, length - read, null);
    if (n === 0) throw new Error('unexpected EOF');
    read += n;
  }
}

while (true) {
  // Read type tag
  const tagBuf = Buffer.alloc(1);
  readExact(tagBuf, 0, 1);
  const tag = tagBuf[0];

  if (tag === TAG_END) {
    // Echo back the end sentinel
    process.stdout.write(Buffer.from([TAG_END]));
    break;
  }

  // Read payload length (u32 LE)
  const lenBuf = Buffer.alloc(4);
  readExact(lenBuf, 0, 4);
  const len = lenBuf.readUInt32LE(0);

  // Read payload
  const payload = Buffer.alloc(len);
  if (len > 0) {
    readExact(payload, 0, len);
  }

  // Decode then re-encode based on type tag
  const reader = new BinaryReader(new Uint8Array(payload));
  let reEncoded: Uint8Array;

  switch (tag) {
    case TAG_POINT: {
      const value = pointCodec.decode(reader);
      const writer = new BinaryWriter(pointCodec.byteSize(value));
      pointCodec.encode(value, writer);
      reEncoded = writer.toUint8Array();
      break;
    }
    case TAG_SHAPE: {
      const value = shapeCodec.decode(reader);
      const writer = new BinaryWriter(shapeCodec.byteSize(value));
      shapeCodec.encode(value, writer);
      reEncoded = writer.toUint8Array();
      break;
    }
    case TAG_MESSAGE: {
      const value = messageCodec.decode(reader);
      const writer = new BinaryWriter(messageCodec.byteSize(value));
      messageCodec.encode(value, writer);
      reEncoded = writer.toUint8Array();
      break;
    }
    default:
      throw new Error(`unknown type tag: ${tag}`);
  }

  // Write response: [tag][length LE][payload]
  const header = Buffer.alloc(5);
  header[0] = tag;
  header.writeUInt32LE(reEncoded.length, 1);
  process.stdout.write(header);
  process.stdout.write(Buffer.from(reEncoded));
}

fs.closeSync(stdin);
