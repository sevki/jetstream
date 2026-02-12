/**
 * r[impl jetstream.wireformat.ts.test-roundtrip]
 * r[impl jetstream.wireformat.ts.test-error]
 */
import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import {
  u8Codec,
  u16Codec,
  u32Codec,
  u64Codec,
  u128Codec,
  i16Codec,
  i32Codec,
  i64Codec,
  i128Codec,
  f32Codec,
  f64Codec,
  boolCodec,
  unitCodec,
} from './primitives.js';
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
 * r[verify jetstream.wireformat.ts.u8]
 * r[verify jetstream.wireformat.u8]
 */
describe('u8', () => {
  test('round-trip 0', () => {
    expect(roundTrip(u8Codec, 0)).toBe(0);
  });
  test('round-trip 255', () => {
    expect(roundTrip(u8Codec, 255)).toBe(255);
  });
  test('round-trip 42', () => {
    expect(roundTrip(u8Codec, 42)).toBe(42);
  });
  test('byteSize is 1', () => {
    expect(u8Codec.byteSize(0)).toBe(1);
  });
  test('encoding is correct', () => {
    const writer = new BinaryWriter();
    u8Codec.encode(0xAB, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0xAB]));
  });
});

/**
 * r[verify jetstream.wireformat.ts.u16]
 * r[verify jetstream.wireformat.u16]
 */
describe('u16', () => {
  test('round-trip 0', () => {
    expect(roundTrip(u16Codec, 0)).toBe(0);
  });
  test('round-trip 65535', () => {
    expect(roundTrip(u16Codec, 65535)).toBe(65535);
  });
  test('round-trip 0x1234', () => {
    expect(roundTrip(u16Codec, 0x1234)).toBe(0x1234);
  });
  test('little-endian encoding', () => {
    const writer = new BinaryWriter();
    u16Codec.encode(0x0102, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0x02, 0x01]));
  });
});

/**
 * r[verify jetstream.wireformat.ts.u32]
 * r[verify jetstream.wireformat.u32]
 */
describe('u32', () => {
  test('round-trip 0', () => {
    expect(roundTrip(u32Codec, 0)).toBe(0);
  });
  test('round-trip max', () => {
    expect(roundTrip(u32Codec, 0xFFFFFFFF)).toBe(0xFFFFFFFF);
  });
  test('little-endian encoding', () => {
    const writer = new BinaryWriter();
    u32Codec.encode(0x01020304, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0x04, 0x03, 0x02, 0x01]));
  });
});

/**
 * r[verify jetstream.wireformat.ts.u64]
 * r[verify jetstream.wireformat.u64]
 */
describe('u64', () => {
  test('round-trip 0', () => {
    expect(roundTrip(u64Codec, 0n)).toBe(0n);
  });
  test('round-trip max', () => {
    expect(roundTrip(u64Codec, 0xFFFFFFFFFFFFFFFFn)).toBe(0xFFFFFFFFFFFFFFFFn);
  });
  test('little-endian encoding', () => {
    const writer = new BinaryWriter();
    u64Codec.encode(0x0102030405060708n, writer);
    expect(writer.toUint8Array()).toEqual(
      new Uint8Array([0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]),
    );
  });
});

/**
 * r[verify jetstream.wireformat.ts.u128]
 * r[verify jetstream.wireformat.u128]
 */
describe('u128', () => {
  test('round-trip 0', () => {
    expect(roundTrip(u128Codec, 0n)).toBe(0n);
  });
  test('round-trip large value', () => {
    const val = (0xDEADBEEFCAFEBABEn << 64n) | 0x0123456789ABCDEFn;
    expect(roundTrip(u128Codec, val)).toBe(val);
  });
  test('round-trip max', () => {
    const max = (1n << 128n) - 1n;
    expect(roundTrip(u128Codec, max)).toBe(max);
  });
  test('encoding: lower 64 bits first, upper 64 bits second', () => {
    const val = (1n << 64n) | 2n; // upper=1, lower=2
    const writer = new BinaryWriter();
    u128Codec.encode(val, writer);
    const bytes = writer.toUint8Array();
    // lower 8 bytes: 2 in LE
    expect(bytes[0]).toBe(2);
    expect(bytes[1]).toBe(0);
    // upper 8 bytes: 1 in LE
    expect(bytes[8]).toBe(1);
    expect(bytes[9]).toBe(0);
  });
});

/**
 * r[verify jetstream.wireformat.ts.i16]
 * r[verify jetstream.wireformat.i16]
 */
describe('i16', () => {
  test('round-trip 0', () => {
    expect(roundTrip(i16Codec, 0)).toBe(0);
  });
  test('round-trip -1', () => {
    expect(roundTrip(i16Codec, -1)).toBe(-1);
  });
  test('round-trip max', () => {
    expect(roundTrip(i16Codec, 32767)).toBe(32767);
  });
  test('round-trip min', () => {
    expect(roundTrip(i16Codec, -32768)).toBe(-32768);
  });
});

/**
 * r[verify jetstream.wireformat.ts.i32]
 * r[verify jetstream.wireformat.i32]
 */
describe('i32', () => {
  test('round-trip 0', () => {
    expect(roundTrip(i32Codec, 0)).toBe(0);
  });
  test('round-trip -1', () => {
    expect(roundTrip(i32Codec, -1)).toBe(-1);
  });
  test('round-trip max', () => {
    expect(roundTrip(i32Codec, 2147483647)).toBe(2147483647);
  });
  test('round-trip min', () => {
    expect(roundTrip(i32Codec, -2147483648)).toBe(-2147483648);
  });
});

/**
 * r[verify jetstream.wireformat.ts.i64]
 * r[verify jetstream.wireformat.i64]
 */
describe('i64', () => {
  test('round-trip 0', () => {
    expect(roundTrip(i64Codec, 0n)).toBe(0n);
  });
  test('round-trip -1', () => {
    expect(roundTrip(i64Codec, -1n)).toBe(-1n);
  });
  test('round-trip large positive', () => {
    expect(roundTrip(i64Codec, 9223372036854775807n)).toBe(9223372036854775807n);
  });
  test('round-trip large negative', () => {
    expect(roundTrip(i64Codec, -9223372036854775808n)).toBe(-9223372036854775808n);
  });
});

/**
 * r[verify jetstream.wireformat.ts.i128]
 * r[verify jetstream.wireformat.i128]
 */
describe('i128', () => {
  test('round-trip 0', () => {
    expect(roundTrip(i128Codec, 0n)).toBe(0n);
  });
  test('round-trip -1', () => {
    expect(roundTrip(i128Codec, -1n)).toBe(-1n);
  });
  test('round-trip positive', () => {
    const val = (1n << 100n);
    expect(roundTrip(i128Codec, val)).toBe(val);
  });
  test('round-trip negative', () => {
    const val = -(1n << 100n);
    expect(roundTrip(i128Codec, val)).toBe(val);
  });
});

/**
 * r[verify jetstream.wireformat.ts.f32]
 * r[verify jetstream.wireformat.f32]
 */
describe('f32', () => {
  test('round-trip 0', () => {
    expect(roundTrip(f32Codec, 0)).toBe(0);
  });
  test('round-trip 1.5', () => {
    expect(roundTrip(f32Codec, 1.5)).toBeCloseTo(1.5, 5);
  });
  test('round-trip -3.14', () => {
    expect(roundTrip(f32Codec, Math.fround(-3.14))).toBeCloseTo(-3.14, 2);
  });
});

/**
 * r[verify jetstream.wireformat.ts.f64]
 * r[verify jetstream.wireformat.f64]
 */
describe('f64', () => {
  test('round-trip 0', () => {
    expect(roundTrip(f64Codec, 0)).toBe(0);
  });
  test('round-trip pi', () => {
    expect(roundTrip(f64Codec, Math.PI)).toBe(Math.PI);
  });
  test('round-trip -1e100', () => {
    expect(roundTrip(f64Codec, -1e100)).toBe(-1e100);
  });
});

/**
 * r[verify jetstream.wireformat.ts.bool]
 * r[verify jetstream.wireformat.bool]
 */
describe('bool', () => {
  test('round-trip true', () => {
    expect(roundTrip(boolCodec, true)).toBe(true);
  });
  test('round-trip false', () => {
    expect(roundTrip(boolCodec, false)).toBe(false);
  });
  test('encoding true = 0x01', () => {
    const writer = new BinaryWriter();
    boolCodec.encode(true, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0x01]));
  });
  test('encoding false = 0x00', () => {
    const writer = new BinaryWriter();
    boolCodec.encode(false, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0x00]));
  });
  test('invalid bool byte throws', () => {
    const reader = new BinaryReader(new Uint8Array([0x02]));
    expect(() => boolCodec.decode(reader)).toThrow('invalid byte for bool');
  });
});

/**
 * r[verify jetstream.wireformat.ts.unit]
 * r[verify jetstream.wireformat.unit]
 */
describe('unit', () => {
  test('byteSize is 0', () => {
    expect(unitCodec.byteSize(undefined)).toBe(0);
  });
  test('round-trip', () => {
    expect(roundTrip(unitCodec, undefined)).toBeUndefined();
  });
});

/**
 * r[verify jetstream.wireformat.ts.test-error]
 */
describe('error handling', () => {
  test('EOF on u8', () => {
    const reader = new BinaryReader(new Uint8Array([]));
    expect(() => u8Codec.decode(reader)).toThrow('unexpected EOF');
  });
  test('EOF on u16', () => {
    const reader = new BinaryReader(new Uint8Array([0x01]));
    expect(() => u16Codec.decode(reader)).toThrow('unexpected EOF');
  });
  test('EOF on u32', () => {
    const reader = new BinaryReader(new Uint8Array([0x01, 0x02]));
    expect(() => u32Codec.decode(reader)).toThrow('unexpected EOF');
  });
  test('EOF on u64', () => {
    const reader = new BinaryReader(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
    expect(() => u64Codec.decode(reader)).toThrow('unexpected EOF');
  });
});
