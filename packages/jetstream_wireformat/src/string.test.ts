import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import { stringCodec } from './string.js';

function roundTrip(value: string): string {
  const writer = new BinaryWriter();
  stringCodec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(stringCodec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return stringCodec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.string]
 * r[verify jetstream.wireformat.string]
 */
describe('string', () => {
  test('round-trip empty string', () => {
    expect(roundTrip('')).toBe('');
  });

  test('round-trip hello', () => {
    expect(roundTrip('hello')).toBe('hello');
  });

  test('round-trip unicode', () => {
    expect(roundTrip('こんにちは')).toBe('こんにちは');
  });

  test('round-trip emoji', () => {
    expect(roundTrip('🚀🎉')).toBe('🚀🎉');
  });

  test('encoding format: u16 length + UTF-8 bytes', () => {
    const writer = new BinaryWriter();
    stringCodec.encode('AB', writer);
    const bytes = writer.toUint8Array();
    // u16 LE length (2) + 'A' (0x41) + 'B' (0x42)
    expect(bytes).toEqual(new Uint8Array([0x02, 0x00, 0x41, 0x42]));
  });

  test('byteSize includes u16 prefix', () => {
    expect(stringCodec.byteSize('')).toBe(2);
    expect(stringCodec.byteSize('A')).toBe(3);
  });

  test('truncated string throws', () => {
    // length says 5 but only 2 bytes available
    const reader = new BinaryReader(new Uint8Array([0x05, 0x00, 0x41, 0x42]));
    expect(() => stringCodec.decode(reader)).toThrow();
  });
});
