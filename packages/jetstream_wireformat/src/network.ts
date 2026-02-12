/**
 * r[impl jetstream.wireformat.ipv4]
 * r[impl jetstream.wireformat.ts.ipv4]
 * r[impl jetstream.wireformat.ipv6]
 * r[impl jetstream.wireformat.ts.ipv6]
 * r[impl jetstream.wireformat.ipaddr]
 * r[impl jetstream.wireformat.ts.ipaddr]
 * r[impl jetstream.wireformat.sockaddr-v4]
 * r[impl jetstream.wireformat.sockaddr-v6]
 * r[impl jetstream.wireformat.sockaddr]
 * r[impl jetstream.wireformat.ts.sockaddr]
 */

import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';

/** IPv4 address as 4 bytes */
export interface IPv4 {
  octets: Uint8Array; // exactly 4 bytes
}

/** IPv6 address as 16 bytes */
export interface IPv6 {
  octets: Uint8Array; // exactly 16 bytes
}

/** IP address discriminated union */
export type IpAddr =
  | { version: 4; addr: IPv4 }
  | { version: 6; addr: IPv6 };

/** Socket address V4 */
export interface SocketAddrV4 {
  ip: IPv4;
  port: number;
}

/** Socket address V6 */
export interface SocketAddrV6 {
  ip: IPv6;
  port: number;
}

/** Socket address discriminated union */
export type SocketAddr =
  | { version: 4; ip: IPv4; port: number }
  | { version: 6; ip: IPv6; port: number };

/** IPv4 codec: 4 raw octets */
export const ipv4Codec: WireFormat<IPv4> = {
  byteSize(_value: IPv4): number {
    return 4;
  },
  encode(value: IPv4, writer: BinaryWriter): void {
    writer.writeBytes(value.octets);
  },
  decode(reader: BinaryReader): IPv4 {
    return { octets: reader.readBytes(4) };
  },
};

/** IPv6 codec: 16 raw octets */
export const ipv6Codec: WireFormat<IPv6> = {
  byteSize(_value: IPv6): number {
    return 16;
  },
  encode(value: IPv6, writer: BinaryWriter): void {
    writer.writeBytes(value.octets);
  },
  decode(reader: BinaryReader): IPv6 {
    return { octets: reader.readBytes(16) };
  },
};

/** IpAddr codec: u8 tag (4=IPv4, 6=IPv6) + address bytes */
export const ipAddrCodec: WireFormat<IpAddr> = {
  byteSize(value: IpAddr): number {
    return 1 + (value.version === 4 ? 4 : 16);
  },
  encode(value: IpAddr, writer: BinaryWriter): void {
    if (value.version === 4) {
      writer.writeU8(4);
      ipv4Codec.encode(value.addr, writer);
    } else {
      writer.writeU8(6);
      ipv6Codec.encode(value.addr, writer);
    }
  },
  decode(reader: BinaryReader): IpAddr {
    const tag = reader.readU8();
    if (tag === 4) {
      return { version: 4, addr: ipv4Codec.decode(reader) };
    }
    if (tag === 6) {
      return { version: 6, addr: ipv6Codec.decode(reader) };
    }
    throw new Error(`Invalid IP address type tag: ${tag}`);
  },
};

/** SocketAddrV4 codec: IPv4 (4 bytes) + port (u16 LE) */
export const socketAddrV4Codec: WireFormat<SocketAddrV4> = {
  byteSize(_value: SocketAddrV4): number {
    return 6;
  },
  encode(value: SocketAddrV4, writer: BinaryWriter): void {
    ipv4Codec.encode(value.ip, writer);
    writer.writeU16(value.port);
  },
  decode(reader: BinaryReader): SocketAddrV4 {
    const ip = ipv4Codec.decode(reader);
    const port = reader.readU16();
    return { ip, port };
  },
};

/** SocketAddrV6 codec: IPv6 (16 bytes) + port (u16 LE) */
export const socketAddrV6Codec: WireFormat<SocketAddrV6> = {
  byteSize(_value: SocketAddrV6): number {
    return 18;
  },
  encode(value: SocketAddrV6, writer: BinaryWriter): void {
    ipv6Codec.encode(value.ip, writer);
    writer.writeU16(value.port);
  },
  decode(reader: BinaryReader): SocketAddrV6 {
    const ip = ipv6Codec.decode(reader);
    const port = reader.readU16();
    return { ip, port };
  },
};

/** SocketAddr codec: u8 tag (4=V4, 6=V6) + typed socket address */
export const socketAddrCodec: WireFormat<SocketAddr> = {
  byteSize(value: SocketAddr): number {
    return 1 + (value.version === 4 ? 6 : 18);
  },
  encode(value: SocketAddr, writer: BinaryWriter): void {
    if (value.version === 4) {
      writer.writeU8(4);
      ipv4Codec.encode(value.ip, writer);
      writer.writeU16(value.port);
    } else {
      writer.writeU8(6);
      ipv6Codec.encode(value.ip, writer);
      writer.writeU16(value.port);
    }
  },
  decode(reader: BinaryReader): SocketAddr {
    const tag = reader.readU8();
    if (tag === 4) {
      const ip = ipv4Codec.decode(reader);
      const port = reader.readU16();
      return { version: 4, ip, port };
    }
    if (tag === 6) {
      const ip = ipv6Codec.decode(reader);
      const port = reader.readU16();
      return { version: 6, ip, port };
    }
    throw new Error(`Invalid socket address type tag: ${tag}`);
  },
};
