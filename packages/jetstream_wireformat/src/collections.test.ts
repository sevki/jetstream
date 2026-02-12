import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import { u16Codec, u32Codec } from './primitives.js';
import { stringCodec } from './string.js';
import { vecCodec, dataCodec, mapCodec, setCodec } from './collections.js';
import type { WireFormat } from './primitives.js';

function roundTrip<T>(codec: WireFormat<T>, value: T): T {
  const writer = new BinaryWriter();
  codec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(codec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return codec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.array]
 * r[verify jetstream.wireformat.vec]
 */
describe('vec', () => {
  const u16Vec = vecCodec(u16Codec);

  test('round-trip empty array', () => {
    expect(roundTrip(u16Vec, [])).toEqual([]);
  });

  test('round-trip [1, 2, 3]', () => {
    expect(roundTrip(u16Vec, [1, 2, 3])).toEqual([1, 2, 3]);
  });

  test('round-trip strings', () => {
    const strVec = vecCodec(stringCodec);
    expect(roundTrip(strVec, ['hello', 'world'])).toEqual(['hello', 'world']);
  });

  test('encoding format: u16 count + elements', () => {
    const writer = new BinaryWriter();
    const u32Vec = vecCodec(u32Codec);
    u32Vec.encode([1], writer);
    const bytes = writer.toUint8Array();
    // u16 LE count (1) + u32 LE value (1)
    expect(bytes).toEqual(new Uint8Array([0x01, 0x00, 0x01, 0x00, 0x00, 0x00]));
  });
});

/**
 * r[verify jetstream.wireformat.ts.data]
 * r[verify jetstream.wireformat.data]
 */
describe('data', () => {
  test('round-trip empty', () => {
    const result = roundTrip(dataCodec, new Uint8Array([]));
    expect(result).toEqual(new Uint8Array([]));
  });

  test('round-trip bytes', () => {
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const result = roundTrip(dataCodec, data);
    expect(result).toEqual(data);
  });

  test('encoding format: u32 length + raw bytes', () => {
    const writer = new BinaryWriter();
    dataCodec.encode(new Uint8Array([0xAA, 0xBB]), writer);
    const bytes = writer.toUint8Array();
    // u32 LE length (2) + raw bytes
    expect(bytes).toEqual(new Uint8Array([0x02, 0x00, 0x00, 0x00, 0xAA, 0xBB]));
  });

  test('rejects data > 32MB', () => {
    // Simulate a length prefix of 33MB
    const buf = new Uint8Array(8);
    const view = new DataView(buf.buffer);
    view.setUint32(0, 33 * 1024 * 1024, true);
    const reader = new BinaryReader(buf);
    expect(() => dataCodec.decode(reader)).toThrow('too large');
  });
});

/**
 * r[verify jetstream.wireformat.ts.map]
 * r[verify jetstream.wireformat.map]
 */
describe('map', () => {
  const strStrMap = mapCodec(
    stringCodec,
    stringCodec,
    (a: string, b: string) => a.localeCompare(b),
  );

  test('round-trip empty map', () => {
    const result = roundTrip(strStrMap, new Map());
    expect(result.size).toBe(0);
  });

  test('round-trip map with entries', () => {
    const map = new Map<string, string>();
    map.set('a', '1');
    map.set('b', '2');
    const result = roundTrip(strStrMap, map);
    expect(result.get('a')).toBe('1');
    expect(result.get('b')).toBe('2');
    expect(result.size).toBe(2);
  });

  test('entries are sorted by key', () => {
    const map = new Map<string, string>();
    map.set('c', '3');
    map.set('a', '1');
    map.set('b', '2');
    const writer = new BinaryWriter();
    strStrMap.encode(map, writer);
    const bytes = writer.toUint8Array();
    const reader = new BinaryReader(bytes);
    // Read count
    expect(reader.readU16()).toBe(3);
    // First key should be 'a' (sorted)
    const firstKeyLen = reader.readU16();
    const firstKeyBytes = reader.readBytes(firstKeyLen);
    expect(new TextDecoder().decode(firstKeyBytes)).toBe('a');
  });
});

/**
 * r[verify jetstream.wireformat.ts.set]
 * r[verify jetstream.wireformat.set]
 */
describe('set', () => {
  const u16Set = setCodec(u16Codec, (a, b) => a - b);

  test('round-trip empty set', () => {
    const result = roundTrip(u16Set, new Set());
    expect(result.size).toBe(0);
  });

  test('round-trip set with elements', () => {
    const result = roundTrip(u16Set, new Set([3, 1, 2]));
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);
    expect(result.size).toBe(3);
  });
});
