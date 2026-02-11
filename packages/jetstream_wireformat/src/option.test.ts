import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import { u32Codec } from './primitives.js';
import type { WireFormat } from './primitives.js';
import { stringCodec } from './string.js';
import { optionCodec } from './option.js';

function roundTrip<T>(codec: WireFormat<T>, value: T): T {
  const writer = new BinaryWriter();
  codec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(codec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return codec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.option]
 * r[verify jetstream.wireformat.option]
 */
describe('option', () => {
  const optU32 = optionCodec(u32Codec);
  const optStr = optionCodec(stringCodec);

  test('round-trip null', () => {
    expect(roundTrip(optU32, null)).toBeNull();
  });

  test('round-trip some value', () => {
    expect(roundTrip(optU32, 42)).toBe(42);
  });

  test('round-trip some string', () => {
    expect(roundTrip(optStr, 'hello')).toBe('hello');
  });

  test('round-trip null string', () => {
    expect(roundTrip(optStr, null)).toBeNull();
  });

  test('None encoding: single 0x00 byte', () => {
    const writer = new BinaryWriter();
    optU32.encode(null, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0x00]));
  });

  test('Some encoding: 0x01 + value', () => {
    const writer = new BinaryWriter();
    optU32.encode(1, writer);
    const bytes = writer.toUint8Array();
    expect(bytes[0]).toBe(0x01);
    // u32 LE 1
    expect(bytes[1]).toBe(0x01);
    expect(bytes[2]).toBe(0x00);
    expect(bytes[3]).toBe(0x00);
    expect(bytes[4]).toBe(0x00);
  });

  test('invalid tag throws', () => {
    const reader = new BinaryReader(new Uint8Array([0x02]));
    expect(() => optU32.decode(reader)).toThrow('Invalid Option tag');
  });
});
