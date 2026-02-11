import { BinaryReader, BinaryWriter, jetStreamErrorCodec, JetStreamError, i32Codec, stringCodec } from '@sevki/jetstream-wireformat';
import type { WireFormat } from '@sevki/jetstream-wireformat';
import { Mux, negotiateVersion } from '@sevki/jetstream-rpc';
import type { Framer, FramerCodec, Context, NegotiatedVersion } from '@sevki/jetstream-rpc';

const MESSAGE_ID_START = 102;
const RERROR = 5;

const TPING: number = 102;
const RPING: number = 103;
const TADD: number = 104;
const RADD: number = 105;

export interface TPing {
  message: string;
}

export const tpingCodec: WireFormat<TPing> = {
  byteSize(value: TPing): number {
    return stringCodec.byteSize(value.message);
  },
  encode(value: TPing, writer: BinaryWriter): void {
    stringCodec.encode(value.message, writer);
  },
  decode(reader: BinaryReader): TPing {
    const message = stringCodec.decode(reader);
    return { message };
  },
};

export interface RPing {
  value: string;
}

export const rpingCodec: WireFormat<RPing> = {
  byteSize(value: RPing): number {
    return stringCodec.byteSize(value.value);
  },
  encode(value: RPing, writer: BinaryWriter): void {
    stringCodec.encode(value.value, writer);
  },
  decode(reader: BinaryReader): RPing {
    return { value: stringCodec.decode(reader) };
  },
};

export interface TAdd {
  a: number;
  b: number;
}

export const taddCodec: WireFormat<TAdd> = {
  byteSize(value: TAdd): number {
    return i32Codec.byteSize(value.a) + i32Codec.byteSize(value.b);
  },
  encode(value: TAdd, writer: BinaryWriter): void {
    i32Codec.encode(value.a, writer);
    i32Codec.encode(value.b, writer);
  },
  decode(reader: BinaryReader): TAdd {
    const a = i32Codec.decode(reader);
    const b = i32Codec.decode(reader);
    return { a, b };
  },
};

export interface RAdd {
  value: number;
}

export const raddCodec: WireFormat<RAdd> = {
  byteSize(value: RAdd): number {
    return i32Codec.byteSize(value.value);
  },
  encode(value: RAdd, writer: BinaryWriter): void {
    i32Codec.encode(value.value, writer);
  },
  decode(reader: BinaryReader): RAdd {
    return { value: i32Codec.decode(reader) };
  },
};

export type Tmessage =
  | { type: 'Ping'; msg: TPing }
  | { type: 'Add'; msg: TAdd };

export type Rmessage =
  | { type: 'Ping'; msg: RPing }
  | { type: 'Add'; msg: RAdd }
  | { type: 'Error'; msg: JetStreamError };

export const tmessageFramer: FramerCodec<Tmessage> = {
  messageType(msg: Tmessage): number {
    switch (msg.type) {
      case 'Ping': return TPING;
      case 'Add': return TADD;
    }
  },
  byteSize(msg: Tmessage): number {
    switch (msg.type) {
      case 'Ping': return tpingCodec.byteSize(msg.msg);
      case 'Add': return taddCodec.byteSize(msg.msg);
    }
  },
  encode(msg: Tmessage, writer: BinaryWriter): void {
    switch (msg.type) {
      case 'Ping': tpingCodec.encode(msg.msg, writer); break;
      case 'Add': taddCodec.encode(msg.msg, writer); break;
    }
  },
  decode(reader: BinaryReader, type: number): Tmessage {
    switch (type) {
      case TPING: return { type: 'Ping', msg: tpingCodec.decode(reader) };
      case TADD: return { type: 'Add', msg: taddCodec.decode(reader) };
      default: throw new Error(`unknown Tmessage type: ${type}`);
    }
  },
};

export const rmessageFramer: FramerCodec<Rmessage> = {
  messageType(msg: Rmessage): number {
    switch (msg.type) {
      case 'Ping': return RPING;
      case 'Add': return RADD;
      case 'Error': return RERROR;
    }
  },
  byteSize(msg: Rmessage): number {
    switch (msg.type) {
      case 'Ping': return rpingCodec.byteSize(msg.msg);
      case 'Add': return raddCodec.byteSize(msg.msg);
      case 'Error': return jetStreamErrorCodec.byteSize(msg.msg);
    }
  },
  encode(msg: Rmessage, writer: BinaryWriter): void {
    switch (msg.type) {
      case 'Ping': rpingCodec.encode(msg.msg, writer); break;
      case 'Add': raddCodec.encode(msg.msg, writer); break;
      case 'Error': jetStreamErrorCodec.encode(msg.msg, writer); break;
    }
  },
  decode(reader: BinaryReader, type: number): Rmessage {
    switch (type) {
      case RPING: return { type: 'Ping', msg: rpingCodec.decode(reader) };
      case RADD: return { type: 'Add', msg: raddCodec.decode(reader) };
      case RERROR: return { type: 'Error', msg: jetStreamErrorCodec.decode(reader) };
      default: throw new Error(`unknown Rmessage type: ${type}`);
    }
  },
};

export class TmessageFramer implements Framer {
  constructor(public readonly inner: Tmessage) {}
  messageType(): number { return tmessageFramer.messageType(this.inner); }
  byteSize(): number { return tmessageFramer.byteSize(this.inner); }
  encode(writer: BinaryWriter): void { tmessageFramer.encode(this.inner, writer); }
}

export class RmessageFramer implements Framer {
  constructor(public readonly inner: Rmessage) {}
  messageType(): number { return rmessageFramer.messageType(this.inner); }
  byteSize(): number { return rmessageFramer.byteSize(this.inner); }
  encode(writer: BinaryWriter): void { rmessageFramer.encode(this.inner, writer); }
}

export function rmessageDecode(reader: BinaryReader, type: number): RmessageFramer {
  return new RmessageFramer(rmessageFramer.decode(reader, type));
}

export const PROTOCOL_NAME = 'rs.jetstream.proto/echohttp';
export const PROTOCOL_VERSION = 'rs.jetstream.proto/echohttp/15.0.0+bfd7d20e';

export class EchoHttpClient {
  private mux: Mux<TmessageFramer, RmessageFramer>;

  constructor(mux: Mux<TmessageFramer, RmessageFramer>) {
    this.mux = mux;
  }

  static async negotiate(
    readable: ReadableStream<Uint8Array>,
    writable: WritableStream<Uint8Array>,
    msize: number = 65536,
  ): Promise<NegotiatedVersion> {
    return negotiateVersion(readable, writable, PROTOCOL_VERSION, msize);
  }

  async ping(message: string): Promise<string> {
    const req = new TmessageFramer({ type: 'Ping', msg: { message } });
    const res = await this.mux.rpc(req);
    if (res.msg.inner.type === 'Error') {
      throw res.msg.inner.msg;
    }
    if (res.msg.inner.type !== 'Ping') {
      throw new Error(`unexpected response type: ${res.msg.inner.type}`);
    }
    return res.msg.inner.msg.value;
  }

  async add(a: number, b: number): Promise<number> {
    const req = new TmessageFramer({ type: 'Add', msg: { a, b } });
    const res = await this.mux.rpc(req);
    if (res.msg.inner.type === 'Error') {
      throw res.msg.inner.msg;
    }
    if (res.msg.inner.type !== 'Add') {
      throw new Error(`unexpected response type: ${res.msg.inner.type}`);
    }
    return res.msg.inner.msg.value;
  }
}

export interface EchoHttpHandler {
  ping(ctx: Context, message: string): Promise<string>;
  add(ctx: Context, a: number, b: number): Promise<number>;
}

export async function dispatchEchoHttp(
  handler: EchoHttpHandler,
  ctx: Context,
  frame: { tag: number; msg: Tmessage },
): Promise<{ tag: number; msg: Rmessage }> {
  try {
    switch (frame.msg.type) {
      case 'Ping': {
        const result = await handler.ping(ctx, frame.msg.msg.message);
        return { tag: frame.tag, msg: { type: 'Ping', msg: { value: result } } };
      }
      case 'Add': {
        const result = await handler.add(ctx, frame.msg.msg.a, frame.msg.msg.b);
        return { tag: frame.tag, msg: { type: 'Add', msg: { value: result } } };
      }
    }
    throw new Error(`unknown message type: ${(frame.msg as any).type}`);
  } catch (err) {
    if (err instanceof JetStreamError) {
      return { tag: frame.tag, msg: { type: 'Error', msg: err } };
    }
    const jsErr = new JetStreamError(
      { message: String(err), code: null, help: null, url: null },
      { internTable: [''], frames: [] },
    );
    return { tag: frame.tag, msg: { type: 'Error', msg: jsErr } };
  }
}
