/**
 * r[impl jetstream.wireformat.string]
 * r[impl jetstream.wireformat.ts.string]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder('utf-8', { fatal: true });

/**
 * String codec: u16 length prefix followed by UTF-8 bytes.
 * Maximum string length is 65,535 bytes (u16::MAX).
 */
export const stringCodec: WireFormat<string> = {
  byteSize(value: string): number {
    return 2 + textEncoder.encode(value).byteLength;
  },
  encode(value: string, writer: BinaryWriter): void {
    const bytes = textEncoder.encode(value);
    if (bytes.byteLength > 0xFFFF) {
      throw new Error(`string is too long: ${bytes.byteLength} bytes exceeds u16 max (65535)`);
    }
    writer.writeU16(bytes.byteLength);
    writer.writeBytes(bytes);
  },
  decode(reader: BinaryReader): string {
    const len = reader.readU16();
    const bytes = reader.readBytes(len);
    return textDecoder.decode(bytes);
  },
};
