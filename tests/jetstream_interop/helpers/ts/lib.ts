import { BinaryReader, BinaryWriter, optionCodec, stringCodec, u32Codec, vecCodec } from '@sevki/jetstream-wireformat';
import type { WireFormat } from '@sevki/jetstream-wireformat';

export interface Point {
  x: number;
  y: number;
}

export const pointCodec: WireFormat<Point> = {
  byteSize(value: Point): number {
    return u32Codec.byteSize(value.x) + u32Codec.byteSize(value.y);
  },
  encode(value: Point, writer: BinaryWriter): void {
    u32Codec.encode(value.x, writer);
    u32Codec.encode(value.y, writer);
  },
  decode(reader: BinaryReader): Point {
    const x = u32Codec.decode(reader);
    const y = u32Codec.decode(reader);
    return { x, y };
  },
};

export interface ColorPoint {
  x: number;
  y: number;
  color: string;
}

export const colorPointCodec: WireFormat<ColorPoint> = {
  byteSize(value: ColorPoint): number {
    return u32Codec.byteSize(value.x) + u32Codec.byteSize(value.y) + stringCodec.byteSize(value.color);
  },
  encode(value: ColorPoint, writer: BinaryWriter): void {
    u32Codec.encode(value.x, writer);
    u32Codec.encode(value.y, writer);
    stringCodec.encode(value.color, writer);
  },
  decode(reader: BinaryReader): ColorPoint {
    const x = u32Codec.decode(reader);
    const y = u32Codec.decode(reader);
    const color = stringCodec.decode(reader);
    return { x, y, color };
  },
};

export type Shape =
  | { tag: 'Circle'; value: number }
  | { tag: 'Rectangle'; width: number; height: number };

export const shapeCodec: WireFormat<Shape> = {
  byteSize(value: Shape): number {
    switch (value.tag) {
      case 'Circle': return 1 + u32Codec.byteSize(value.value);
      case 'Rectangle': return 1 + u32Codec.byteSize(value.width) + u32Codec.byteSize(value.height);
    }
  },
  encode(value: Shape, writer: BinaryWriter): void {
    switch (value.tag) {
      case 'Circle': writer.writeU8(0); u32Codec.encode(value.value, writer); break;
      case 'Rectangle': writer.writeU8(1); u32Codec.encode(value.width, writer); u32Codec.encode(value.height, writer); break;
    }
  },
  decode(reader: BinaryReader): Shape {
    const tag = reader.readU8();
    switch (tag) {
      case 0: return { tag: 'Circle', value: u32Codec.decode(reader) };
      case 1: { const width = u32Codec.decode(reader); const height = u32Codec.decode(reader); return { tag: 'Rectangle', width, height }; }
      default: throw new Error(`invalid variant index: ${tag}`);
    }
  },
};

export interface Message {
  id: number;
  tags: string[];
  payload: string | null;
}

export const messageCodec: WireFormat<Message> = {
  byteSize(value: Message): number {
    return u32Codec.byteSize(value.id) + vecCodec(stringCodec).byteSize(value.tags) + optionCodec(stringCodec).byteSize(value.payload);
  },
  encode(value: Message, writer: BinaryWriter): void {
    u32Codec.encode(value.id, writer);
    vecCodec(stringCodec).encode(value.tags, writer);
    optionCodec(stringCodec).encode(value.payload, writer);
  },
  decode(reader: BinaryReader): Message {
    const id = u32Codec.decode(reader);
    const tags = vecCodec(stringCodec).decode(reader);
    const payload = optionCodec(stringCodec).decode(reader);
    return { id, tags, payload };
  },
};

