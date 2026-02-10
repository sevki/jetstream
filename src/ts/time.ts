/**
 * r[impl jetstream.wireformat.systime]
 * r[impl jetstream.wireformat.ts.systime]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

/**
 * SystemTime codec: u64 milliseconds since Unix epoch (LE).
 * Represented as a JavaScript Date object.
 */
export const systemTimeCodec: WireFormat<Date> = {
  byteSize(_value: Date): number {
    return 8;
  },
  encode(value: Date, writer: BinaryWriter): void {
    const millis = BigInt(value.getTime());
    writer.writeU64(millis);
  },
  decode(reader: BinaryReader): Date {
    const millis = reader.readU64();
    // Safe for Date: up to 2^53-1 ms (~285,000 years)
    const millisNum = Number(millis);
    if (millis > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new Error('timestamp overflow: milliseconds exceed safe integer range');
    }
    return new Date(millisNum);
  },
};
