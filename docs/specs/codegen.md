# JetStream Codegen Specification

This document specifies the code generation library that transforms Rust types annotated with `#[derive(JetStreamWireFormat)]` into equivalent TypeScript and Swift types with WireFormat codec implementations.

## Intermediate Representation

r[jetstream.codegen.ir]
The codegen library uses typeshare-core IR types (RustStruct, RustEnum, RustType, SpecialRustType) as the intermediate representation between Rust source and target language output.

r[jetstream.codegen.parser]
The codegen parser transforms `syn::DeriveInput` from `#[derive(JetStreamWireFormat)]` annotated types into typeshare-core IR (RustStruct or RustEnum).

## TypeScript Backend

r[jetstream.codegen.ts]
The TypeScript backend implements the typeshare-core `Language` trait to generate WireFormat codec code from the IR.

r[jetstream.codegen.ts.struct]
For each Rust struct, the TypeScript backend generates a TypeScript `interface` declaration and a `const WireFormat<T>` codec object providing `byteSize(value)`, `encode(value, writer)`, and `decode(reader)` methods.

r[jetstream.codegen.ts.enum]
For each Rust enum, the TypeScript backend generates a TypeScript discriminated union type and a codec object that dispatches on a `u8` variant tag index for encoding and decoding.

## Swift Backend

r[jetstream.codegen.swift]
The Swift backend implements the typeshare-core `Language` trait to generate types conforming to the WireFormat protocol from the IR.

r[jetstream.codegen.swift.struct]
For each Rust struct, the Swift backend generates a Swift `struct` conforming to the `WireFormat` protocol, providing `byteSize`, `encode(to:)`, and `static decode(from:)` implementations.

r[jetstream.codegen.swift.enum]
For each Rust enum, the Swift backend generates a Swift `enum` conforming to `WireFormat` with `u8` variant index dispatch for encoding and decoding.

## Field Attributes

r[jetstream.codegen.skip]
Fields annotated with `#[jetstream(skip)]` are omitted from the generated encode and decode logic. On decode, skipped fields are set to their type's default value.

## Type Mapping

r[jetstream.codegen.type-map]
The codegen maps Rust types to target language types as follows: `u8` → `number` / `UInt8`, `u16` → `number` / `UInt16`, `u32` → `number` / `UInt32`, `u64` → `bigint` / `UInt64`, `i8` → `number` / `Int8`, `i16` → `number` / `Int16`, `i32` → `number` / `Int32`, `i64` → `bigint` / `Int64`, `u128` → `bigint` / `UInt128`, `i128` → `bigint` / `Int128`, `f32` → `number` / `Float`, `f64` → `number` / `Double`, `bool` → `boolean` / `Bool`, `String` → `string` / `String`, `Vec<T>` → `T[]` / `[SwiftT]`, `Option<T>` → `T | null` / `SwiftT?`, `Box<T>` → transparent unwrap (encodes as inner type).

## Service Definitions

r[jetstream.codegen.service]
The codegen parses `#[service]` trait definitions into a `ServiceDef` IR containing the service name, methods with their parameter types, return types, and assigned message IDs.

r[jetstream.codegen.service.ts]
For each service, the TypeScript backend generates request/response interfaces, `Tmessage`/`Rmessage` codec objects, an `EchoClient` class with async methods per service method, and an `EchoHandler` interface that consumers implement.

r[jetstream.codegen.service.swift]
For each service, the Swift backend generates request/response structs, `Tmessage`/`Rmessage` enums, a client class with async methods per service method, and a handler protocol that consumers conform to.
