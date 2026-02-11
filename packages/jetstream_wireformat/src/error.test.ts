import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';
import {
  Level,
  levelCodec,
  fieldPairCodec,
  frameCodec,
  backtraceCodec,
  errorInnerCodec,
  JetStreamError,
  jetStreamErrorCodec,
} from './error.js';
import type { FieldPair, Frame, Backtrace, ErrorInner } from './error.js';

function roundTrip<T>(codec: WireFormat<T>, value: T): T {
  const writer = new BinaryWriter();
  codec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(codec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return codec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.level]
 * r[verify jetstream.wireformat.level]
 */
describe('level', () => {
  test('round-trip TRACE', () => {
    expect(roundTrip(levelCodec, Level.TRACE)).toBe(Level.TRACE);
  });
  test('round-trip DEBUG', () => {
    expect(roundTrip(levelCodec, Level.DEBUG)).toBe(Level.DEBUG);
  });
  test('round-trip INFO', () => {
    expect(roundTrip(levelCodec, Level.INFO)).toBe(Level.INFO);
  });
  test('round-trip WARN', () => {
    expect(roundTrip(levelCodec, Level.WARN)).toBe(Level.WARN);
  });
  test('round-trip ERROR', () => {
    expect(roundTrip(levelCodec, Level.ERROR)).toBe(Level.ERROR);
  });
  test('invalid level throws', () => {
    const reader = new BinaryReader(new Uint8Array([5]));
    expect(() => levelCodec.decode(reader)).toThrow('Invalid Level value');
  });
  test('encoding TRACE = 0', () => {
    const writer = new BinaryWriter();
    levelCodec.encode(Level.TRACE, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([0]));
  });
  test('encoding ERROR = 4', () => {
    const writer = new BinaryWriter();
    levelCodec.encode(Level.ERROR, writer);
    expect(writer.toUint8Array()).toEqual(new Uint8Array([4]));
  });
});

/**
 * r[verify jetstream.wireformat.ts.fieldpair]
 * r[verify jetstream.wireformat.fieldpair]
 */
describe('fieldPair', () => {
  test('round-trip', () => {
    const pair: FieldPair = { key: 1, value: 2 };
    const result = roundTrip(fieldPairCodec, pair);
    expect(result.key).toBe(1);
    expect(result.value).toBe(2);
  });
  test('byteSize is 4', () => {
    expect(fieldPairCodec.byteSize({ key: 0, value: 0 })).toBe(4);
  });
});

/**
 * r[verify jetstream.wireformat.ts.frame]
 * r[verify jetstream.wireformat.frame]
 */
describe('frame', () => {
  test('round-trip minimal frame', () => {
    const frame: Frame = {
      msg: 'test_span',
      name: 0,
      target: 1,
      module: 2,
      file: 3,
      line: 42,
      fields: [],
      level: Level.INFO,
    };
    const result = roundTrip(frameCodec, frame);
    expect(result.msg).toBe('test_span');
    expect(result.name).toBe(0);
    expect(result.target).toBe(1);
    expect(result.module).toBe(2);
    expect(result.file).toBe(3);
    expect(result.line).toBe(42);
    expect(result.fields).toEqual([]);
    expect(result.level).toBe(Level.INFO);
  });

  test('round-trip frame with fields', () => {
    const frame: Frame = {
      msg: 'with_fields',
      name: 1,
      target: 2,
      module: 3,
      file: 4,
      line: 100,
      fields: [{ key: 5, value: 6 }],
      level: Level.ERROR,
    };
    const result = roundTrip(frameCodec, frame);
    expect(result.fields.length).toBe(1);
    expect(result.fields[0].key).toBe(5);
    expect(result.fields[0].value).toBe(6);
    expect(result.level).toBe(Level.ERROR);
  });
});

/**
 * r[verify jetstream.wireformat.ts.backtrace]
 * r[verify jetstream.wireformat.backtrace]
 */
describe('backtrace', () => {
  test('round-trip empty backtrace', () => {
    const bt: Backtrace = { internTable: [''], frames: [] };
    const result = roundTrip(backtraceCodec, bt);
    expect(result.internTable).toEqual(['']);
    expect(result.frames).toEqual([]);
  });

  test('round-trip backtrace with frames', () => {
    const bt: Backtrace = {
      internTable: ['', 'myFunc', 'myModule', 'main.rs'],
      frames: [
        {
          msg: 'processing request',
          name: 1,
          target: 2,
          module: 2,
          file: 3,
          line: 55,
          fields: [],
          level: Level.DEBUG,
        },
      ],
    };
    const result = roundTrip(backtraceCodec, bt);
    expect(result.internTable.length).toBe(4);
    expect(result.internTable[0]).toBe('');
    expect(result.internTable[1]).toBe('myFunc');
    expect(result.frames.length).toBe(1);
    expect(result.frames[0].msg).toBe('processing request');
    expect(result.frames[0].line).toBe(55);
  });
});

/**
 * r[verify jetstream.wireformat.ts.error-inner]
 * r[verify jetstream.wireformat.error-inner]
 */
describe('errorInner', () => {
  test('round-trip with all fields null', () => {
    const inner: ErrorInner = {
      message: 'something failed',
      code: null,
      help: null,
      url: null,
    };
    const result = roundTrip(errorInnerCodec, inner);
    expect(result.message).toBe('something failed');
    expect(result.code).toBeNull();
    expect(result.help).toBeNull();
    expect(result.url).toBeNull();
  });

  test('round-trip with all fields present', () => {
    const inner: ErrorInner = {
      message: 'connection refused',
      code: 'E001',
      help: 'Check your network connection',
      url: 'https://example.com/errors/E001',
    };
    const result = roundTrip(errorInnerCodec, inner);
    expect(result.message).toBe('connection refused');
    expect(result.code).toBe('E001');
    expect(result.help).toBe('Check your network connection');
    expect(result.url).toBe('https://example.com/errors/E001');
  });
});

/**
 * r[verify jetstream.wireformat.ts.error]
 * r[verify jetstream.wireformat.error]
 */
describe('jetStreamError', () => {
  test('round-trip minimal error', () => {
    const err = new JetStreamError(
      { message: 'test error', code: null, help: null, url: null },
      { internTable: [''], frames: [] },
    );
    const result = roundTrip(jetStreamErrorCodec, err);
    expect(result).toBeInstanceOf(JetStreamError);
    expect(result.inner.message).toBe('test error');
    expect(result.message).toBe('test error');
    expect(result.backtrace.frames).toEqual([]);
  });

  test('round-trip error with backtrace', () => {
    const err = new JetStreamError(
      {
        message: 'not found',
        code: '404',
        help: 'Resource does not exist',
        url: null,
      },
      {
        internTable: ['', 'handler', 'server', 'main.rs'],
        frames: [
          {
            msg: 'handle_request',
            name: 1,
            target: 2,
            module: 2,
            file: 3,
            line: 120,
            fields: [{ key: 1, value: 2 }],
            level: Level.WARN,
          },
        ],
      },
    );
    const result = roundTrip(jetStreamErrorCodec, err);
    expect(result.inner.code).toBe('404');
    expect(result.inner.help).toBe('Resource does not exist');
    expect(result.backtrace.frames.length).toBe(1);
    expect(result.backtrace.frames[0].fields.length).toBe(1);
  });
});

/**
 * r[verify jetstream.wireformat.ts.test-roundtrip]
 */
describe('comprehensive round-trip tests', () => {
  test('error with unicode message', () => {
    const err = new JetStreamError(
      { message: 'エラー: 接続が拒否されました', code: null, help: null, url: null },
      { internTable: [''], frames: [] },
    );
    const result = roundTrip(jetStreamErrorCodec, err);
    expect(result.inner.message).toBe('エラー: 接続が拒否されました');
  });
});
