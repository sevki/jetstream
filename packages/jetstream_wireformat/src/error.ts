/**
 * r[impl jetstream.wireformat.error-inner]
 * r[impl jetstream.wireformat.ts.error-inner]
 * r[impl jetstream.wireformat.backtrace]
 * r[impl jetstream.wireformat.ts.backtrace]
 * r[impl jetstream.wireformat.frame]
 * r[impl jetstream.wireformat.ts.frame]
 * r[impl jetstream.wireformat.fieldpair]
 * r[impl jetstream.wireformat.ts.fieldpair]
 * r[impl jetstream.wireformat.level]
 * r[impl jetstream.wireformat.ts.level]
 * r[impl jetstream.wireformat.error]
 * r[impl jetstream.wireformat.ts.error]
 */

import { BinaryReader } from "./binary-reader.js";
import { BinaryWriter } from "./binary-writer.js";
import type { WireFormat } from "./primitives.js";
import { u16Codec } from "./primitives.js";
import { stringCodec } from "./string.js";
import { optionCodec } from "./option.js";
import { vecCodec } from "./collections.js";

/** Tracing Level enum */
export enum Level {
  TRACE = 0,
  DEBUG = 1,
  INFO = 2,
  WARN = 3,
  ERROR = 4,
}

/** Level codec: u8 with custom mapping (NOT standard u8 WireFormat) */
export const levelCodec: WireFormat<Level> = {
  byteSize(_value: Level): number {
    return 1;
  },
  encode(value: Level, writer: BinaryWriter): void {
    writer.writeU8(value);
  },
  decode(reader: BinaryReader): Level {
    const byte = reader.readU8();
    if (byte > 4) {
      throw new Error(`Invalid Level value: ${byte}`);
    }
    return byte as Level;
  },
};

/** FieldPair: key and value as u16 intern table indices */
export interface FieldPair {
  key: number; // u16 intern table index
  value: number; // u16 intern table index
}

export const fieldPairCodec: WireFormat<FieldPair> = {
  byteSize(_value: FieldPair): number {
    return 4;
  },
  encode(value: FieldPair, writer: BinaryWriter): void {
    u16Codec.encode(value.key, writer);
    u16Codec.encode(value.value, writer);
  },
  decode(reader: BinaryReader): FieldPair {
    const key = u16Codec.decode(reader);
    const value = u16Codec.decode(reader);
    return { key, value };
  },
};

/** Frame: a single backtrace frame */
export interface Frame {
  msg: string; // span name (WF-STRING)
  name: number; // u16 intern table index
  target: number; // u16 intern table index
  module: number; // u16 intern table index
  file: number; // u16 intern table index
  line: number; // u16 source line number
  fields: FieldPair[];
  level: Level;
}

const fieldPairVecCodec = vecCodec(fieldPairCodec);

export const frameCodec: WireFormat<Frame> = {
  byteSize(value: Frame): number {
    return (
      stringCodec.byteSize(value.msg) +
      2 + // name
      2 + // target
      2 + // module
      2 + // file
      2 + // line
      fieldPairVecCodec.byteSize(value.fields) +
      1
    ); // level
  },
  encode(value: Frame, writer: BinaryWriter): void {
    stringCodec.encode(value.msg, writer);
    u16Codec.encode(value.name, writer);
    u16Codec.encode(value.target, writer);
    u16Codec.encode(value.module, writer);
    u16Codec.encode(value.file, writer);
    u16Codec.encode(value.line, writer);
    fieldPairVecCodec.encode(value.fields, writer);
    levelCodec.encode(value.level, writer);
  },
  decode(reader: BinaryReader): Frame {
    const msg = stringCodec.decode(reader);
    const name = u16Codec.decode(reader);
    const target = u16Codec.decode(reader);
    const module = u16Codec.decode(reader);
    const file = u16Codec.decode(reader);
    const line = u16Codec.decode(reader);
    const fields = fieldPairVecCodec.decode(reader);
    const level = levelCodec.decode(reader);
    return { msg, name, target, module, file, line, fields, level };
  },
};

/** Backtrace with intern table */
export interface Backtrace {
  internTable: string[]; // index 0 = "" (empty string)
  frames: Frame[];
}

const stringVecCodec = vecCodec(stringCodec);
const frameVecCodec = vecCodec(frameCodec);

export const backtraceCodec: WireFormat<Backtrace> = {
  byteSize(value: Backtrace): number {
    return (
      stringVecCodec.byteSize(value.internTable) +
      frameVecCodec.byteSize(value.frames)
    );
  },
  encode(value: Backtrace, writer: BinaryWriter): void {
    stringVecCodec.encode(value.internTable, writer);
    frameVecCodec.encode(value.frames, writer);
  },
  decode(reader: BinaryReader): Backtrace {
    const internTable = stringVecCodec.decode(reader);
    const frames = frameVecCodec.decode(reader);
    return { internTable, frames };
  },
};

/** ErrorInner: message + optional code, help, url */
export interface ErrorInner {
  message: string;
  code: string | null;
  help: string | null;
  url: string | null;
}

const optionalStringCodec = optionCodec(stringCodec);

export const errorInnerCodec: WireFormat<ErrorInner> = {
  byteSize(value: ErrorInner): number {
    return (
      stringCodec.byteSize(value.message) +
      optionalStringCodec.byteSize(value.code) +
      optionalStringCodec.byteSize(value.help) +
      optionalStringCodec.byteSize(value.url)
    );
  },
  encode(value: ErrorInner, writer: BinaryWriter): void {
    stringCodec.encode(value.message, writer);
    optionalStringCodec.encode(value.code, writer);
    optionalStringCodec.encode(value.help, writer);
    optionalStringCodec.encode(value.url, writer);
  },
  decode(reader: BinaryReader): ErrorInner {
    const message = stringCodec.decode(reader);
    const code = optionalStringCodec.decode(reader);
    const help = optionalStringCodec.decode(reader);
    const url = optionalStringCodec.decode(reader);
    return { message, code, help, url };
  },
};

/** JetStreamError: ErrorInner + Backtrace, extends Error */
export class JetStreamError extends Error {
  inner: ErrorInner;
  backtrace: Backtrace;

  constructor(inner: ErrorInner, backtrace: Backtrace) {
    super(inner.message);
    this.name = "JetStreamError";
    this.inner = inner;
    this.backtrace = backtrace;
  }
}

export const jetStreamErrorCodec: WireFormat<JetStreamError> = {
  byteSize(value: JetStreamError): number {
    return (
      errorInnerCodec.byteSize(value.inner) +
      backtraceCodec.byteSize(value.backtrace)
    );
  },
  encode(value: JetStreamError, writer: BinaryWriter): void {
    errorInnerCodec.encode(value.inner, writer);
    backtraceCodec.encode(value.backtrace, writer);
  },
  decode(reader: BinaryReader): JetStreamError {
    const inner = errorInnerCodec.decode(reader);
    const backtrace = backtraceCodec.decode(reader);
    return new JetStreamError(inner, backtrace);
  },
};
