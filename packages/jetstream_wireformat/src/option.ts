/**
 * r[impl jetstream.wireformat.option]
 * r[impl jetstream.wireformat.ts.option]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

/**
 * Creates an Option codec: u8 tag (0=None/null, 1=Some/present).
 * Represented as T | null in TypeScript.
 */
export function optionCodec<T>(innerCodec: WireFormat<T>): WireFormat<T | null> {
  return {
    byteSize(value: T | null): number {
      if (value === null) return 1;
      return 1 + innerCodec.byteSize(value);
    },
    encode(value: T | null, writer: BinaryWriter): void {
      if (value === null) {
        writer.writeU8(0x00);
      } else {
        writer.writeU8(0x01);
        innerCodec.encode(value, writer);
      }
    },
    decode(reader: BinaryReader): T | null {
      const tag = reader.readU8();
      if (tag === 0) return null;
      if (tag === 1) return innerCodec.decode(reader);
      throw new Error(`Invalid Option tag: ${tag}`);
    },
  };
}
