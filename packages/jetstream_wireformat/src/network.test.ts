import { describe, test, expect } from 'vitest';
import { BinaryReader } from './binary-reader.js';
import { BinaryWriter } from './binary-writer.js';
import type { WireFormat } from './primitives.js';
import {
  ipv4Codec,
  ipv6Codec,
  ipAddrCodec,
  socketAddrV4Codec,
  socketAddrV6Codec,
  socketAddrCodec,
} from './network.js';
import type { IPv4, IPv6, IpAddr, SocketAddr } from './network.js';

function roundTrip<T>(codec: WireFormat<T>, value: T): T {
  const writer = new BinaryWriter();
  codec.encode(value, writer);
  const bytes = writer.toUint8Array();
  expect(bytes.byteLength).toBe(codec.byteSize(value));
  const reader = new BinaryReader(bytes);
  return codec.decode(reader);
}

/**
 * r[verify jetstream.wireformat.ts.ipv4]
 * r[verify jetstream.wireformat.ipv4]
 */
describe('ipv4', () => {
  test('round-trip 192.168.1.1', () => {
    const ip: IPv4 = { octets: new Uint8Array([192, 168, 1, 1]) };
    const result = roundTrip(ipv4Codec, ip);
    expect(result.octets).toEqual(new Uint8Array([192, 168, 1, 1]));
  });

  test('round-trip 0.0.0.0', () => {
    const ip: IPv4 = { octets: new Uint8Array([0, 0, 0, 0]) };
    const result = roundTrip(ipv4Codec, ip);
    expect(result.octets).toEqual(new Uint8Array([0, 0, 0, 0]));
  });

  test('byteSize is 4', () => {
    const ip: IPv4 = { octets: new Uint8Array([1, 2, 3, 4]) };
    expect(ipv4Codec.byteSize(ip)).toBe(4);
  });
});

/**
 * r[verify jetstream.wireformat.ts.ipv6]
 * r[verify jetstream.wireformat.ipv6]
 */
describe('ipv6', () => {
  test('round-trip loopback', () => {
    const octets = new Uint8Array(16);
    octets[15] = 1; // ::1
    const ip: IPv6 = { octets };
    const result = roundTrip(ipv6Codec, ip);
    expect(result.octets).toEqual(octets);
  });

  test('byteSize is 16', () => {
    const ip: IPv6 = { octets: new Uint8Array(16) };
    expect(ipv6Codec.byteSize(ip)).toBe(16);
  });
});

/**
 * r[verify jetstream.wireformat.ts.ipaddr]
 * r[verify jetstream.wireformat.ipaddr]
 */
describe('ipaddr', () => {
  test('round-trip IPv4', () => {
    const addr: IpAddr = { version: 4, addr: { octets: new Uint8Array([10, 0, 0, 1]) } };
    const result = roundTrip(ipAddrCodec, addr);
    expect(result.version).toBe(4);
    if (result.version === 4) {
      expect(result.addr.octets).toEqual(new Uint8Array([10, 0, 0, 1]));
    }
  });

  test('round-trip IPv6', () => {
    const octets = new Uint8Array(16);
    octets[0] = 0xFE;
    octets[1] = 0x80;
    const addr: IpAddr = { version: 6, addr: { octets } };
    const result = roundTrip(ipAddrCodec, addr);
    expect(result.version).toBe(6);
  });

  test('tag 4 for IPv4', () => {
    const addr: IpAddr = { version: 4, addr: { octets: new Uint8Array([1, 2, 3, 4]) } };
    const writer = new BinaryWriter();
    ipAddrCodec.encode(addr, writer);
    expect(writer.toUint8Array()[0]).toBe(4);
  });

  test('tag 6 for IPv6', () => {
    const addr: IpAddr = { version: 6, addr: { octets: new Uint8Array(16) } };
    const writer = new BinaryWriter();
    ipAddrCodec.encode(addr, writer);
    expect(writer.toUint8Array()[0]).toBe(6);
  });

  test('invalid tag throws', () => {
    const reader = new BinaryReader(new Uint8Array([5, 0, 0, 0, 0]));
    expect(() => ipAddrCodec.decode(reader)).toThrow('Invalid IP address type tag');
  });
});

/**
 * r[verify jetstream.wireformat.ts.sockaddr]
 * r[verify jetstream.wireformat.sockaddr-v4]
 * r[verify jetstream.wireformat.sockaddr-v6]
 * r[verify jetstream.wireformat.sockaddr]
 */
describe('socketaddr', () => {
  test('round-trip V4', () => {
    const addr: SocketAddr = {
      version: 4,
      ip: { octets: new Uint8Array([127, 0, 0, 1]) },
      port: 8080,
    };
    const result = roundTrip(socketAddrCodec, addr);
    expect(result.version).toBe(4);
    expect(result.port).toBe(8080);
    if (result.version === 4) {
      expect(result.ip.octets).toEqual(new Uint8Array([127, 0, 0, 1]));
    }
  });

  test('round-trip V6', () => {
    const octets = new Uint8Array(16);
    octets[15] = 1;
    const addr: SocketAddr = {
      version: 6,
      ip: { octets },
      port: 443,
    };
    const result = roundTrip(socketAddrCodec, addr);
    expect(result.version).toBe(6);
    expect(result.port).toBe(443);
  });

  test('V4 byteSize is 7 (1 tag + 4 ip + 2 port)', () => {
    const addr: SocketAddr = {
      version: 4,
      ip: { octets: new Uint8Array([0, 0, 0, 0]) },
      port: 0,
    };
    expect(socketAddrCodec.byteSize(addr)).toBe(7);
  });

  test('V6 byteSize is 19 (1 tag + 16 ip + 2 port)', () => {
    const addr: SocketAddr = {
      version: 6,
      ip: { octets: new Uint8Array(16) },
      port: 0,
    };
    expect(socketAddrCodec.byteSize(addr)).toBe(19);
  });

  test('invalid tag throws', () => {
    const reader = new BinaryReader(new Uint8Array([5]));
    expect(() => socketAddrCodec.decode(reader)).toThrow('Invalid socket address type tag');
  });

  test('round-trip SocketAddrV4 codec', () => {
    const addr = { ip: { octets: new Uint8Array([10, 0, 0, 1]) }, port: 3000 };
    const result = roundTrip(socketAddrV4Codec, addr);
    expect(result.ip.octets).toEqual(new Uint8Array([10, 0, 0, 1]));
    expect(result.port).toBe(3000);
  });

  test('round-trip SocketAddrV6 codec', () => {
    const octets = new Uint8Array(16);
    octets[0] = 0xFF;
    const addr = { ip: { octets }, port: 9090 };
    const result = roundTrip(socketAddrV6Codec, addr);
    expect(result.ip.octets[0]).toBe(0xFF);
    expect(result.port).toBe(9090);
  });
});
