/**
 * r[impl jetstream.wireformat.ts.reader]
 * r[impl jetstream.wireformat.byte-order]
 */

/**
 * BinaryReader reads from a Uint8Array with a cursor position.
 * All multi-byte reads use little-endian byte order per 9P2000.L conventions.
 */
export class BinaryReader {
  private buffer: Uint8Array;
  private view: DataView;
  private offset: number;

  constructor(buffer: Uint8Array) {
    this.buffer = buffer;
    this.view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    this.offset = 0;
  }

  get remaining(): number {
    return this.buffer.byteLength - this.offset;
  }

  readBytes(count: number): Uint8Array {
    if (this.offset + count > this.buffer.byteLength) {
      throw new Error(`unexpected EOF: wanted ${count} bytes, have ${this.remaining}`);
    }
    const result = this.buffer.slice(this.offset, this.offset + count);
    this.offset += count;
    return result;
  }

  readU8(): number {
    if (this.offset + 1 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 1 byte');
    }
    const value = this.view.getUint8(this.offset);
    this.offset += 1;
    return value;
  }

  readU16(): number {
    if (this.offset + 2 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 2 bytes');
    }
    const value = this.view.getUint16(this.offset, true);
    this.offset += 2;
    return value;
  }

  readU32(): number {
    if (this.offset + 4 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 4 bytes');
    }
    const value = this.view.getUint32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readU64(): bigint {
    if (this.offset + 8 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 8 bytes');
    }
    const value = this.view.getBigUint64(this.offset, true);
    this.offset += 8;
    return value;
  }

  readI16(): number {
    if (this.offset + 2 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 2 bytes');
    }
    const value = this.view.getInt16(this.offset, true);
    this.offset += 2;
    return value;
  }

  readI32(): number {
    if (this.offset + 4 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 4 bytes');
    }
    const value = this.view.getInt32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readI64(): bigint {
    if (this.offset + 8 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 8 bytes');
    }
    const value = this.view.getBigInt64(this.offset, true);
    this.offset += 8;
    return value;
  }

  readF32(): number {
    if (this.offset + 4 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 4 bytes');
    }
    const value = this.view.getFloat32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readF64(): number {
    if (this.offset + 8 > this.buffer.byteLength) {
      throw new Error('unexpected EOF: wanted 8 bytes');
    }
    const value = this.view.getFloat64(this.offset, true);
    this.offset += 8;
    return value;
  }
}
