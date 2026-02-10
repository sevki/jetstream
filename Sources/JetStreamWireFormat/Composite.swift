// JetStream WireFormat — Composite Type Helpers
// Copyright (c) 2024, Sevki <s@sevki.io>
// SPDX-License-Identifier: BSD-3-Clause

import Foundation

// r[impl jetstream.wireformat.struct]
// r[impl jetstream.wireformat.swift.struct]
// Structs conforming to WireFormat should encode/decode fields sequentially
// in declaration order. No length prefix or field names on the wire.
//
// Example:
//   struct MyStruct: WireFormat {
//       var x: UInt32
//       var y: String
//
//       func byteSize() -> UInt32 {
//           x.byteSize() + y.byteSize()
//       }
//       func encode(writer: inout BinaryWriter) throws {
//           try x.encode(writer: &writer)
//           try y.encode(writer: &writer)
//       }
//       static func decode(reader: inout BinaryReader) throws -> MyStruct {
//           let x = try UInt32.decode(reader: &reader)
//           let y = try String.decode(reader: &reader)
//           return MyStruct(x: x, y: y)
//       }
//   }

// r[impl jetstream.wireformat.enum]
// r[impl jetstream.wireformat.swift.enum]
// Enums conforming to WireFormat should use a u8 variant index (0-based)
// followed by the variant's fields encoded sequentially.
//
// Example:
//   enum MyEnum: WireFormat {
//       case a             // variant 0
//       case b(UInt32)     // variant 1
//       case c(String)     // variant 2
//
//       func byteSize() -> UInt32 {
//           switch self {
//           case .a: return 1
//           case .b(let v): return 1 + v.byteSize()
//           case .c(let s): return 1 + s.byteSize()
//           }
//       }
//       func encode(writer: inout BinaryWriter) throws {
//           switch self {
//           case .a:
//               writer.writeU8(0)
//           case .b(let v):
//               writer.writeU8(1)
//               try v.encode(writer: &writer)
//           case .c(let s):
//               writer.writeU8(2)
//               try s.encode(writer: &writer)
//           }
//       }
//       static func decode(reader: inout BinaryReader) throws -> MyEnum {
//           let variant = try reader.readU8()
//           switch variant {
//           case 0: return .a
//           case 1: return .b(try UInt32.decode(reader: &reader))
//           case 2: return .c(try String.decode(reader: &reader))
//           default: throw WireFormatError.invalidEnumVariant(variant)
//           }
//       }
//   }
