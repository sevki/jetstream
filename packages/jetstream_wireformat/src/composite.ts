/**
 * r[impl jetstream.wireformat.struct]
 * r[impl jetstream.wireformat.ts.struct]
 * r[impl jetstream.wireformat.enum]
 * r[impl jetstream.wireformat.ts.enum]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

/**
 * FieldCodec describes a single struct field for sequential encoding.
 */
export interface FieldCodec<T, F> {
  codec: WireFormat<F>;
  get(value: T): F;
  set(target: Partial<T>, fieldValue: F): void;
}

/**
 * Creates a struct codec that encodes fields sequentially in declaration order.
 * No length prefix, no field names on the wire.
 */
export function structCodec<T>(
  fields: FieldCodec<T, any>[],
  construct: (partial: Partial<T>) => T,
): WireFormat<T> {
  return {
    byteSize(value: T): number {
      let size = 0;
      for (const field of fields) {
        size += field.codec.byteSize(field.get(value));
      }
      return size;
    },
    encode(value: T, writer: BinaryWriter): void {
      for (const field of fields) {
        field.codec.encode(field.get(value), writer);
      }
    },
    decode(reader: BinaryReader): T {
      const partial: Partial<T> = {};
      for (const field of fields) {
        const fieldValue = field.codec.decode(reader);
        field.set(partial, fieldValue);
      }
      return construct(partial);
    },
  };
}

/**
 * VariantCodec describes a single enum variant.
 */
export interface VariantCodec<T> {
  index: number;
  match(value: T): boolean;
  encode(value: T, writer: BinaryWriter): void;
  decode(reader: BinaryReader): T;
  byteSize(value: T): number;
}

/**
 * Creates an enum codec using u8 variant index.
 * Each variant is matched and encoded/decoded according to its definition.
 */
export function enumCodec<T>(variants: VariantCodec<T>[]): WireFormat<T> {
  return {
    byteSize(value: T): number {
      for (const variant of variants) {
        if (variant.match(value)) {
          return 1 + variant.byteSize(value);
        }
      }
      throw new Error('no matching variant for value');
    },
    encode(value: T, writer: BinaryWriter): void {
      for (const variant of variants) {
        if (variant.match(value)) {
          writer.writeU8(variant.index);
          variant.encode(value, writer);
          return;
        }
      }
      throw new Error('no matching variant for value');
    },
    decode(reader: BinaryReader): T {
      const index = reader.readU8();
      for (const variant of variants) {
        if (variant.index === index) {
          return variant.decode(reader);
        }
      }
      throw new Error(`invalid variant index: ${index}`);
    },
  };
}
