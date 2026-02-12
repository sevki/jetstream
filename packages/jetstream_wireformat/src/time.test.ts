/**
 * r[impl jetstream.wireformat.ts.test-compat]
 */
import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';
import { systemTimeCodec } from './time.js';

function roundTrip<T>(codec: WireFormat<T>, value: T): T {
  const writer = new BinaryWriter();
  codec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(codec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return codec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.systime]
 * r[verify jetstream.wireformat.systime]
 */
describe('systemTime', () => {
  test('round-trip epoch', () => {
    const date = new Date(0);
    const result = roundTrip(systemTimeCodec, date);
    expect(result.getTime()).toBe(0);
  });

  test('round-trip now', () => {
    const date = new Date();
    const result = roundTrip(systemTimeCodec, date);
    expect(result.getTime()).toBe(date.getTime());
  });

  test('round-trip specific date', () => {
    const date = new Date('2024-06-15T12:00:00Z');
    const result = roundTrip(systemTimeCodec, date);
    expect(result.getTime()).toBe(date.getTime());
  });

  test('byteSize is 8', () => {
    expect(systemTimeCodec.byteSize(new Date())).toBe(8);
  });

  test('encoding is u64 LE millis', () => {
    const date = new Date(256); // 256 ms
    const writer = new BinaryWriter();
    systemTimeCodec.encode(date, writer);
    const bytes = writer.toUint8Array();
    // 256 in LE u64 = [0x00, 0x01, 0, 0, 0, 0, 0, 0]
    expect(bytes[0]).toBe(0x00);
    expect(bytes[1]).toBe(0x01);
    expect(bytes[2]).toBe(0x00);
  });
});
