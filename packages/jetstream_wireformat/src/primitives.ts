/**
 * r[impl jetstream.wireformat.ts.interface]
 * r[impl jetstream.wireformat.trait]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';

/**
 * WireFormat interface - every type that can be transmitted over the wire
 * MUST implement this interface providing three operations.
 */
export interface WireFormat<T> {
  byteSize(value: T): number;
  encode(value: T, writer: BinaryWriter): void;
  decode(reader: BinaryReader): T;
}

/**
 * r[impl jetstream.wireformat.u8]
 * r[impl jetstream.wireformat.ts.u8]
 */
export const u8Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 1;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeU8(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readU8();
  },
};

/**
 * r[impl jetstream.wireformat.u16]
 * r[impl jetstream.wireformat.ts.u16]
 */
export const u16Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 2;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeU16(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readU16();
  },
};

/**
 * r[impl jetstream.wireformat.u32]
 * r[impl jetstream.wireformat.ts.u32]
 */
export const u32Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 4;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeU32(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readU32();
  },
};

/**
 * r[impl jetstream.wireformat.u64]
 * r[impl jetstream.wireformat.ts.u64]
 */
export const u64Codec: WireFormat<bigint> = {
  byteSize(_value: bigint): number {
    return 8;
  },
  encode(value: bigint, writer: BinaryWriter): void {
    writer.writeU64(value);
  },
  decode(reader: BinaryReader): bigint {
    return reader.readU64();
  },
};

/**
 * r[impl jetstream.wireformat.u128]
 * r[impl jetstream.wireformat.ts.u128]
 */
export const u128Codec: WireFormat<bigint> = {
  byteSize(_value: bigint): number {
    return 16;
  },
  encode(value: bigint, writer: BinaryWriter): void {
    const mask64 = 0xFFFFFFFFFFFFFFFFn;
    const lower = value & mask64;
    const upper = (value >> 64n) & mask64;
    writer.writeU64(lower);
    writer.writeU64(upper);
  },
  decode(reader: BinaryReader): bigint {
    const lower = reader.readU64();
    const upper = reader.readU64();
    return (upper << 64n) | lower;
  },
};

/**
 * r[impl jetstream.wireformat.i16]
 * r[impl jetstream.wireformat.ts.i16]
 */
export const i16Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 2;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeI16(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readI16();
  },
};

/**
 * r[impl jetstream.wireformat.i32]
 * r[impl jetstream.wireformat.ts.i32]
 */
export const i32Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 4;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeI32(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readI32();
  },
};

/**
 * r[impl jetstream.wireformat.i64]
 * r[impl jetstream.wireformat.ts.i64]
 */
export const i64Codec: WireFormat<bigint> = {
  byteSize(_value: bigint): number {
    return 8;
  },
  encode(value: bigint, writer: BinaryWriter): void {
    writer.writeI64(value);
  },
  decode(reader: BinaryReader): bigint {
    return reader.readI64();
  },
};

/**
 * r[impl jetstream.wireformat.i128]
 * r[impl jetstream.wireformat.ts.i128]
 */
export const i128Codec: WireFormat<bigint> = {
  byteSize(_value: bigint): number {
    return 16;
  },
  encode(value: bigint, writer: BinaryWriter): void {
    const mask64 = 0xFFFFFFFFFFFFFFFFn;
    // Lower 64 bits as unsigned
    const lower = value & mask64;
    writer.writeU64(lower);
    // Upper 64 bits as signed
    const upper = value >> 64n;
    writer.writeI64(upper);
  },
  decode(reader: BinaryReader): bigint {
    // Lower 64 bits as unsigned
    const lower = reader.readU64();
    // Upper 64 bits as signed
    const upper = reader.readI64();
    return (upper << 64n) | (lower & 0xFFFFFFFFFFFFFFFFn);
  },
};

/**
 * r[impl jetstream.wireformat.f32]
 * r[impl jetstream.wireformat.ts.f32]
 */
export const f32Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 4;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeF32(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readF32();
  },
};

/**
 * r[impl jetstream.wireformat.f64]
 * r[impl jetstream.wireformat.ts.f64]
 */
export const f64Codec: WireFormat<number> = {
  byteSize(_value: number): number {
    return 8;
  },
  encode(value: number, writer: BinaryWriter): void {
    writer.writeF64(value);
  },
  decode(reader: BinaryReader): number {
    return reader.readF64();
  },
};

/**
 * r[impl jetstream.wireformat.bool]
 * r[impl jetstream.wireformat.ts.bool]
 */
export const boolCodec: WireFormat<boolean> = {
  byteSize(_value: boolean): number {
    return 1;
  },
  encode(value: boolean, writer: BinaryWriter): void {
    writer.writeU8(value ? 0x01 : 0x00);
  },
  decode(reader: BinaryReader): boolean {
    const byte = reader.readU8();
    if (byte === 0) return false;
    if (byte === 1) return true;
    throw new Error(`invalid byte for bool: ${byte}`);
  },
};

/**
 * r[impl jetstream.wireformat.unit]
 * r[impl jetstream.wireformat.ts.unit]
 */
export const unitCodec: WireFormat<undefined> = {
  byteSize(_value: undefined): number {
    return 0;
  },
  encode(_value: undefined, _writer: BinaryWriter): void {
    // nothing to write
  },
  decode(_reader: BinaryReader): undefined {
    return undefined;
  },
};
