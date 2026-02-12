/**
 * r[impl jetstream.rpc.ts.frame]
 * r[impl jetstream.rpc.ts.framer]
 */

import type { WireFormat } from '@sevki/jetstream-wireformat';
import { BinaryReader } from '@sevki/jetstream-wireformat';
import { BinaryWriter } from '@sevki/jetstream-wireformat';
import { u8Codec, u16Codec, u32Codec } from '@sevki/jetstream-wireformat';

export interface Framer {
  messageType(): number;
  byteSize(): number;
  encode(writer: BinaryWriter): void;
}

export type FramerDecode<T extends Framer> = (reader: BinaryReader, type: number) => T;

export interface Frame<T extends Framer> {
  tag: number;
  msg: T;
}

/**
 * Frame wire format: [size:u32 LE][type:u8][tag:u16 LE][payload]
 * size includes itself (min 7 bytes = 4 + 1 + 2)
 */
export function frameCodec<T extends Framer>(decode: FramerDecode<T>): WireFormat<Frame<T>> {
  return {
    byteSize(value: Frame<T>): number {
      return 4 + 1 + 2 + value.msg.byteSize();
    },
    encode(value: Frame<T>, writer: BinaryWriter): void {
      const totalSize = 4 + 1 + 2 + value.msg.byteSize();
      u32Codec.encode(totalSize, writer);
      u8Codec.encode(value.msg.messageType(), writer);
      u16Codec.encode(value.tag, writer);
      value.msg.encode(writer);
    },
    decode(reader: BinaryReader): Frame<T> {
      const size = u32Codec.decode(reader);
      if (size < 4) throw new Error(`frame size ${size} < 4`);
      const type = u8Codec.decode(reader);
      const tag = u16Codec.decode(reader);
      const msg = decode(reader, type);
      return { tag, msg };
    },
  };
}
