/**
 * r[impl jetstream.wireformat.vec]
 * r[impl jetstream.wireformat.ts.array]
 * r[impl jetstream.wireformat.data]
 * r[impl jetstream.wireformat.ts.data]
 * r[impl jetstream.wireformat.map]
 * r[impl jetstream.wireformat.ts.map]
 * r[impl jetstream.wireformat.set]
 * r[impl jetstream.wireformat.ts.set]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

const MAX_DATA_LENGTH = 32 * 1024 * 1024; // 32 MB

/**
 * Creates a Vec/Array codec with u16 count prefix.
 * Max 65,535 elements.
 */
export function vecCodec<T>(elementCodec: WireFormat<T>): WireFormat<T[]> {
  return {
    byteSize(value: T[]): number {
      let size = 2; // u16 count
      for (const elem of value) {
        size += elementCodec.byteSize(elem);
      }
      return size;
    },
    encode(value: T[], writer: BinaryWriter): void {
      if (value.length > 0xFFFF) {
        throw new Error(`too many elements in vector: ${value.length} exceeds u16 max (65535)`);
      }
      writer.writeU16(value.length);
      for (const elem of value) {
        elementCodec.encode(elem, writer);
      }
    },
    decode(reader: BinaryReader): T[] {
      const len = reader.readU16();
      const result: T[] = [];
      for (let i = 0; i < len; i++) {
        result.push(elementCodec.decode(reader));
      }
      return result;
    },
  };
}

/**
 * Data codec: u32 length prefix followed by raw bytes.
 * Max 32 MB (33,554,432 bytes).
 */
export const dataCodec: WireFormat<Uint8Array> = {
  byteSize(value: Uint8Array): number {
    return 4 + value.byteLength;
  },
  encode(value: Uint8Array, writer: BinaryWriter): void {
    writer.writeU32(value.byteLength);
    writer.writeBytes(value);
  },
  decode(reader: BinaryReader): Uint8Array {
    const len = reader.readU32();
    if (len > MAX_DATA_LENGTH) {
      throw new Error(`data length (${len} bytes) is too large, max is ${MAX_DATA_LENGTH}`);
    }
    return reader.readBytes(len);
  },
};

/**
 * Creates a Map codec with u16 count prefix.
 * Entries are encoded in sorted key order to match Rust BTreeMap.
 * Max 65,535 entries.
 */
export function mapCodec<K, V>(
  keyCodec: WireFormat<K>,
  valueCodec: WireFormat<V>,
  compareKeys: (a: K, b: K) => number,
): WireFormat<Map<K, V>> {
  return {
    byteSize(value: Map<K, V>): number {
      let size = 2; // u16 count
      for (const [k, v] of value) {
        size += keyCodec.byteSize(k) + valueCodec.byteSize(v);
      }
      return size;
    },
    encode(value: Map<K, V>, writer: BinaryWriter): void {
      if (value.size > 0xFFFF) {
        throw new Error(`map too large: ${value.size} exceeds u16 max (65535)`);
      }
      writer.writeU16(value.size);
      // Sort entries by key to match BTreeMap
      const entries = [...value.entries()].sort((a, b) => compareKeys(a[0], b[0]));
      for (const [k, v] of entries) {
        keyCodec.encode(k, writer);
        valueCodec.encode(v, writer);
      }
    },
    decode(reader: BinaryReader): Map<K, V> {
      const len = reader.readU16();
      const map = new Map<K, V>();
      for (let i = 0; i < len; i++) {
        const key = keyCodec.decode(reader);
        const value = valueCodec.decode(reader);
        map.set(key, value);
      }
      return map;
    },
  };
}

/**
 * Creates a Set codec with u16 count prefix.
 * Elements are encoded in sorted order to match Rust BTreeSet.
 * Max 65,535 elements.
 */
export function setCodec<T>(
  elementCodec: WireFormat<T>,
  compareElements: (a: T, b: T) => number,
): WireFormat<Set<T>> {
  return {
    byteSize(value: Set<T>): number {
      let size = 2; // u16 count
      for (const elem of value) {
        size += elementCodec.byteSize(elem);
      }
      return size;
    },
    encode(value: Set<T>, writer: BinaryWriter): void {
      if (value.size > 0xFFFF) {
        throw new Error(`set too large: ${value.size} exceeds u16 max (65535)`);
      }
      writer.writeU16(value.size);
      const sorted = [...value].sort(compareElements);
      for (const elem of sorted) {
        elementCodec.encode(elem, writer);
      }
    },
    decode(reader: BinaryReader): Set<T> {
      const len = reader.readU16();
      const set = new Set<T>();
      for (let i = 0; i < len; i++) {
        set.add(elementCodec.decode(reader));
      }
      return set;
    },
  };
}
