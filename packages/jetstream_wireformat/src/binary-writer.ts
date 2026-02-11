/**
 * r[impl jetstream.wireformat.ts.writer]
 * r[impl jetstream.wireformat.byte-order]
 */

/**
 * BinaryWriter writes to a growable buffer.
 * All multi-byte writes use little-endian byte order per 9P2000.L conventions.
 */
export class BinaryWriter {
  private buffer: Uint8Array;
  private view: DataView;
  private offset: number;

  constructor(initialCapacity: number = 256) {
    this.buffer = new Uint8Array(initialCapacity);
    this.view = new DataView(this.buffer.buffer);
    this.offset = 0;
  }

  private ensureCapacity(additionalBytes: number): void {
    const required = this.offset + additionalBytes;
    if (required <= this.buffer.byteLength) return;
    let newCapacity = this.buffer.byteLength;
    while (newCapacity < required) {
      newCapacity *= 2;
    }
    const newBuffer = new Uint8Array(newCapacity);
    newBuffer.set(this.buffer);
    this.buffer = newBuffer;
    this.view = new DataView(this.buffer.buffer);
  }

  writeBytes(bytes: Uint8Array): void {
    this.ensureCapacity(bytes.byteLength);
    this.buffer.set(bytes, this.offset);
    this.offset += bytes.byteLength;
  }

  writeU8(value: number): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.offset, value);
    this.offset += 1;
  }

  writeU16(value: number): void {
    this.ensureCapacity(2);
    this.view.setUint16(this.offset, value, true);
    this.offset += 2;
  }

  writeU32(value: number): void {
    this.ensureCapacity(4);
    this.view.setUint32(this.offset, value, true);
    this.offset += 4;
  }

  writeU64(value: bigint): void {
    this.ensureCapacity(8);
    this.view.setBigUint64(this.offset, value, true);
    this.offset += 8;
  }

  writeI16(value: number): void {
    this.ensureCapacity(2);
    this.view.setInt16(this.offset, value, true);
    this.offset += 2;
  }

  writeI32(value: number): void {
    this.ensureCapacity(4);
    this.view.setInt32(this.offset, value, true);
    this.offset += 4;
  }

  writeI64(value: bigint): void {
    this.ensureCapacity(8);
    this.view.setBigInt64(this.offset, value, true);
    this.offset += 8;
  }

  writeF32(value: number): void {
    this.ensureCapacity(4);
    this.view.setFloat32(this.offset, value, true);
    this.offset += 4;
  }

  writeF64(value: number): void {
    this.ensureCapacity(8);
    this.view.setFloat64(this.offset, value, true);
    this.offset += 8;
  }

  toUint8Array(): Uint8Array {
    return this.buffer.slice(0, this.offset);
  }
}
