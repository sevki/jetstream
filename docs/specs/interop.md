# JetStream Cross-Language Interop Test Specification

This document specifies the cross-language interoperability testing framework that verifies TypeScript and Swift implementations produce byte-identical wire format output to the Rust reference implementation.

## Test Protocol

r[jetstream.interop.protocol]
The interop test protocol communicates over stdin/stdout with the following frame format: `[u8 type_tag][u32 LE payload_length][payload bytes]`. A type tag of `0xFF` signals end-of-test. The type tag identifies which test type is being round-tripped.

## Round-Trip Test

r[jetstream.interop.roundtrip]
In a round-trip test, the Rust test driver encodes a random value and sends the encoded bytes to the child process. The child process decodes the bytes into its native type, re-encodes it, and sends the re-encoded bytes back. The Rust driver then asserts the returned bytes are identical to the original.

## Byte Identity

r[jetstream.interop.byte-identical]
Re-encoded bytes from the target language (TypeScript or Swift) MUST exactly match the Rust-encoded bytes. Any difference in byte output for the same logical value constitutes a test failure. This ensures all implementations are wire-compatible.

## TypeScript Interop

r[jetstream.interop.ts]
TypeScript interop tests run via a Node.js child process spawned by the Rust test driver. The Rust driver communicates with the Node.js process over stdin/stdout using the interop protocol.

## Swift Interop

r[jetstream.interop.swift]
Swift interop tests run via a podman container child process spawned by the Rust test driver. The Rust driver communicates with the containerized Swift process over stdin/stdout using the interop protocol.

## Property-Based Testing

r[jetstream.interop.proptest]
The Rust test driver uses proptest to generate random instances of test types for fuzzing. Each generated value is serialized and sent through the round-trip protocol, ensuring correctness across a wide range of inputs rather than only hand-crafted test cases.
