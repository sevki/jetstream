// r[impl jetstream.codegen.service.swift]

use std::fmt::Write;

use convert_case::{Case, Casing};

use crate::service_parser::{MethodDef, ServiceDef};
use crate::swift_backend::{rust_type_to_swift, SwiftConfig};

/// r[jetstream.rpc.swift.message-ids]
/// Generate Swift RPC code from a ServiceDef.
///
/// This generates:
/// - Request/response structs conforming to WireFormat
/// - Tmessage/Rmessage enums with Framer conformance
/// - Client class with async methods
/// - Handler protocol
pub fn generate_swift_rpc(
    service: &ServiceDef,
    config: &SwiftConfig,
) -> String {
    let mut out = String::new();

    writeln!(out, "import Foundation").unwrap();
    writeln!(out, "import {}", config.wireformat_module).unwrap();
    writeln!(out, "import JetStreamRpc").unwrap();
    writeln!(out).unwrap();

    let name = &service.name;

    // Message ID constants
    writeln!(out, "private let MESSAGE_ID_START: UInt8 = 102").unwrap();
    writeln!(out, "private let RERROR: UInt8 = 5").unwrap();
    writeln!(out).unwrap();

    for method in &service.methods {
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "private let T{upper}: UInt8 = {}", method.request_id)
            .unwrap();
        writeln!(out, "private let R{upper}: UInt8 = {}", method.response_id)
            .unwrap();
    }
    writeln!(out).unwrap();

    // Request/response structs
    for method in &service.methods {
        generate_swift_message_types(&mut out, method);
    }

    // r[jetstream.rpc.swift.framer]
    // Tmessage enum
    generate_swift_tmessage(&mut out, &service.methods);
    writeln!(out).unwrap();

    // Rmessage enum
    generate_swift_rmessage(&mut out, &service.methods);
    writeln!(out).unwrap();

    // Protocol name and version constants
    let name_lower = name.to_lowercase();
    let version = if service.version.is_empty() {
        "0.0.0"
    } else {
        &service.version
    };
    let digest = &service.digest;
    writeln!(
        out,
        "public let PROTOCOL_NAME = \"rs.jetstream.proto/{name_lower}\"",
    )
    .unwrap();
    writeln!(
        out,
        "public let PROTOCOL_VERSION = \"rs.jetstream.proto/{name_lower}/{version}+{digest}\"",
    )
    .unwrap();
    writeln!(out).unwrap();

    // Client class
    generate_swift_client(&mut out, name, &service.methods);
    writeln!(out).unwrap();

    // Handler protocol
    generate_swift_handler(&mut out, name, &service.methods);

    out
}

fn generate_swift_message_types(out: &mut String, method: &MethodDef) {
    let pascal = method.name.to_case(Case::Pascal);

    // Request struct
    writeln!(out, "public struct T{pascal}: WireFormat {{").unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        let swift_type = rust_type_to_swift(&p.ty);
        writeln!(out, "    public var {field_name}: {swift_type}").unwrap();
    }
    writeln!(out).unwrap();

    // byteSize
    writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
    if method.params.is_empty() {
        writeln!(out, "        return 0").unwrap();
    } else {
        let parts: Vec<String> = method
            .params
            .iter()
            .map(|p| format!("{}.byteSize()", p.name.to_case(Case::Camel)))
            .collect();
        writeln!(out, "        return {}", parts.join(" + ")).unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // encode
    writeln!(
        out,
        "    public func encode(writer: inout BinaryWriter) throws {{"
    )
    .unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        writeln!(out, "        try {field_name}.encode(writer: &writer)")
            .unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // decode
    writeln!(
        out,
        "    public static func decode(reader: inout BinaryReader) throws -> T{pascal} {{"
    )
    .unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        let swift_type = rust_type_to_swift(&p.ty);
        writeln!(
            out,
            "        let {field_name} = try {swift_type}.decode(reader: &reader)"
        )
        .unwrap();
    }
    let field_list: Vec<String> = method
        .params
        .iter()
        .map(|p| {
            let field_name = p.name.to_case(Case::Camel);
            format!("{field_name}: {field_name}")
        })
        .collect();
    writeln!(out, "        return T{pascal}({})", field_list.join(", "))
        .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // Response struct
    if let Some(ret_type) = &method.return_type {
        let swift_type = rust_type_to_swift(ret_type);
        writeln!(out, "public struct R{pascal}: WireFormat {{").unwrap();
        writeln!(out, "    public var value: {swift_type}").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
        writeln!(out, "        return value.byteSize()").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "    public func encode(writer: inout BinaryWriter) throws {{"
        )
        .unwrap();
        writeln!(out, "        try value.encode(writer: &writer)").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "    public static func decode(reader: inout BinaryReader) throws -> R{pascal} {{"
        )
        .unwrap();
        writeln!(
            out,
            "        let value = try {swift_type}.decode(reader: &reader)"
        )
        .unwrap();
        writeln!(out, "        return R{pascal}(value: value)").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
    } else {
        writeln!(out, "public struct R{pascal}: WireFormat {{").unwrap();
        writeln!(out, "    public func byteSize() -> UInt32 {{ return 0 }}")
            .unwrap();
        writeln!(
            out,
            "    public func encode(writer: inout BinaryWriter) throws {{ }}"
        )
        .unwrap();
        writeln!(
            out,
            "    public static func decode(reader: inout BinaryReader) throws -> R{pascal} {{ return R{pascal}() }}"
        )
        .unwrap();
        writeln!(out, "}}").unwrap();
    }
    writeln!(out).unwrap();
}

fn generate_swift_tmessage(out: &mut String, methods: &[MethodDef]) {
    writeln!(out, "public enum Tmessage: Framer {{").unwrap();

    // Cases
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);
        writeln!(out, "    case {case_name}(T{pascal})").unwrap();
    }
    writeln!(out).unwrap();

    // messageType
    writeln!(out, "    public func messageType() -> UInt8 {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "        case .{case_name}: return T{upper}").unwrap();
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // byteSize
    writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "        case .{case_name}(let msg): return msg.byteSize()"
        )
        .unwrap();
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // encode
    writeln!(
        out,
        "    public func encode(writer: inout BinaryWriter) throws {{"
    )
    .unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        writeln!(out, "        case .{case_name}(let msg):").unwrap();
        writeln!(out, "            try msg.encode(writer: &writer)").unwrap();
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // decode
    writeln!(
        out,
        "    public static func decode(reader: inout BinaryReader, type ty: UInt8) throws -> Tmessage {{"
    )
    .unwrap();
    writeln!(out, "        switch ty {{").unwrap();
    for method in methods {
        let upper = method.name.to_case(Case::UpperSnake);
        let case_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);
        writeln!(
            out,
            "        case T{upper}: return .{case_name}(try T{pascal}.decode(reader: &reader))"
        )
        .unwrap();
    }
    writeln!(
        out,
        "        default: throw WireFormatError.invalidMessageType(ty)"
    )
    .unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();

    writeln!(out, "}}").unwrap();
}

fn generate_swift_rmessage(out: &mut String, methods: &[MethodDef]) {
    // r[jetstream.rpc.swift.error-frame]
    writeln!(out, "public enum Rmessage: Framer {{").unwrap();

    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);
        writeln!(out, "    case {case_name}(R{pascal})").unwrap();
    }
    writeln!(out, "    case error(JetStreamError)").unwrap();
    writeln!(out).unwrap();

    // messageType
    writeln!(out, "    public func messageType() -> UInt8 {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "        case .{case_name}: return R{upper}").unwrap();
    }
    writeln!(out, "        case .error: return RERROR").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // byteSize
    writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "        case .{case_name}(let msg): return msg.byteSize()"
        )
        .unwrap();
    }
    writeln!(out, "        case .error(let err): return err.byteSize()")
        .unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // encode
    writeln!(
        out,
        "    public func encode(writer: inout BinaryWriter) throws {{"
    )
    .unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        writeln!(out, "        case .{case_name}(let msg):").unwrap();
        writeln!(out, "            try msg.encode(writer: &writer)").unwrap();
    }
    writeln!(out, "        case .error(let err):").unwrap();
    writeln!(out, "            try err.encode(writer: &writer)").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // decode
    writeln!(
        out,
        "    public static func decode(reader: inout BinaryReader, type ty: UInt8) throws -> Rmessage {{"
    )
    .unwrap();
    writeln!(out, "        switch ty {{").unwrap();
    for method in methods {
        let upper = method.name.to_case(Case::UpperSnake);
        let case_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);
        writeln!(
            out,
            "        case R{upper}: return .{case_name}(try R{pascal}.decode(reader: &reader))"
        )
        .unwrap();
    }
    writeln!(
        out,
        "        case RERROR: return .error(try JetStreamError.decode(reader: &reader))"
    )
    .unwrap();
    writeln!(
        out,
        "        default: throw WireFormatError.invalidMessageType(ty)"
    )
    .unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();

    writeln!(out, "}}").unwrap();
}

// r[impl jetstream.rpc.swift.client]
fn generate_swift_client(
    out: &mut String,
    service_name: &str,
    methods: &[MethodDef],
) {
    let client_name = format!("{service_name}Client");

    writeln!(out, "public class {client_name} {{").unwrap();
    writeln!(out, "    private let mux: Mux").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    public init(mux: Mux) {{").unwrap();
    writeln!(out, "        self.mux = mux").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // r[impl jetstream.version.framer.client-handshake]
    // Static method to negotiate version using raw frame I/O
    writeln!(
        out,
        "    // r[impl jetstream.version.framer.client-handshake]"
    )
    .unwrap();
    writeln!(out, "    /// Perform Tversion/Rversion handshake.").unwrap();
    writeln!(
        out,
        "    /// Call before creating a Mux on the same stream."
    )
    .unwrap();
    writeln!(out, "    public static func negotiate(").unwrap();
    writeln!(out, "        writer: inout BinaryWriter,").unwrap();
    writeln!(out, "        reader: inout BinaryReader,").unwrap();
    writeln!(out, "        msize: UInt32 = 65536").unwrap();
    writeln!(out, "    ) throws -> NegotiatedVersion {{").unwrap();
    writeln!(out, "        let tversion = Tversion(msize: msize, version: PROTOCOL_VERSION)").unwrap();
    writeln!(
        out,
        "        let totalSize: UInt32 = 4 + 1 + 2 + tversion.byteSize()"
    )
    .unwrap();
    writeln!(out, "        writer.writeU32(totalSize)").unwrap();
    writeln!(out, "        writer.writeU8(TVERSION)").unwrap();
    writeln!(out, "        writer.writeU16(0xFFFF) // NOTAG").unwrap();
    writeln!(out, "        try tversion.encode(writer: &writer)").unwrap();
    writeln!(out, "        let size = try reader.readU32()").unwrap();
    writeln!(out, "        guard size >= 7 else {{ throw VersionNegotiationError.streamClosed }}").unwrap();
    writeln!(out, "        let ty = try reader.readU8()").unwrap();
    writeln!(out, "        let _ = try reader.readU16() // tag").unwrap();
    writeln!(out, "        guard ty == RVERSION else {{ throw VersionNegotiationError.unexpectedMessageType(ty) }}").unwrap();
    writeln!(
        out,
        "        let rversion = try Rversion.decode(reader: &reader)"
    )
    .unwrap();
    writeln!(out, "        guard rversion.version != \"unknown\" else {{ throw VersionNegotiationError.rejected }}").unwrap();
    writeln!(out, "        return NegotiatedVersion(msize: rversion.msize, version: rversion.version)").unwrap();
    writeln!(out, "    }}").unwrap();

    for method in methods {
        let method_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);

        let params: Vec<String> = method
            .params
            .iter()
            .map(|p| {
                let name = p.name.to_case(Case::Camel);
                let swift_type = rust_type_to_swift(&p.ty);
                format!("{name}: {swift_type}")
            })
            .collect();

        let ret_type = method
            .return_type
            .as_ref()
            .map(rust_type_to_swift)
            .unwrap_or_else(|| "Void".into());

        writeln!(out).unwrap();
        writeln!(
            out,
            "    public func {method_name}({}) async throws -> {ret_type} {{",
            params.join(", ")
        )
        .unwrap();

        let args: Vec<String> = method
            .params
            .iter()
            .map(|p| {
                let name = p.name.to_case(Case::Camel);
                format!("{name}: {name}")
            })
            .collect();
        writeln!(
            out,
            "        let req = Tmessage.{method_name}(T{pascal}({}))",
            args.join(", ")
        )
        .unwrap();
        writeln!(out, "        let res = try await mux.rpc(req)").unwrap();
        writeln!(out, "        switch res {{").unwrap();
        writeln!(out, "        case .{method_name}(let msg):").unwrap();
        if method.return_type.is_some() {
            writeln!(out, "            return msg.value").unwrap();
        } else {
            writeln!(out, "            return").unwrap();
        }
        writeln!(out, "        case .error(let err):").unwrap();
        writeln!(out, "            throw err").unwrap();
        writeln!(out, "        default:").unwrap();
        writeln!(
            out,
            "            throw WireFormatError.invalidMessageType(0)"
        )
        .unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
    }

    writeln!(out, "}}").unwrap();
}

// r[impl jetstream.rpc.swift.handler]
fn generate_swift_handler(
    out: &mut String,
    service_name: &str,
    methods: &[MethodDef],
) {
    let handler_name = format!("{service_name}Handler");

    writeln!(out, "public protocol {handler_name}: Sendable {{").unwrap();
    for method in methods {
        let method_name = method.name.to_case(Case::Camel);

        let mut all_params = vec!["ctx: Context".to_string()];
        all_params.extend(method.params.iter().map(|p| {
            let name = p.name.to_case(Case::Camel);
            let swift_type = rust_type_to_swift(&p.ty);
            format!("{name}: {swift_type}")
        }));

        let ret_type = method
            .return_type
            .as_ref()
            .map(rust_type_to_swift)
            .unwrap_or_else(|| "Void".into());

        writeln!(
            out,
            "    func {method_name}({}) async throws -> {ret_type}",
            all_params.join(", ")
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // r[impl jetstream.rpc.swift.handler.dispatch]
    generate_swift_dispatch(out, service_name, methods);
}

fn generate_swift_dispatch(
    out: &mut String,
    service_name: &str,
    methods: &[MethodDef],
) {
    let handler_name = format!("{service_name}Handler");

    writeln!(
        out,
        "public func dispatch{service_name}<H: {handler_name}>(handler: H, ctx: Context, frame: Frame<Tmessage>) async -> Frame<Rmessage> {{"
    )
    .unwrap();
    writeln!(out, "    let tag = frame.tag").unwrap();
    writeln!(out, "    do {{").unwrap();
    writeln!(out, "        switch frame.msg {{").unwrap();

    for method in methods {
        let case_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);

        writeln!(out, "        case .{case_name}(let req):").unwrap();

        // Build the call arguments: ctx first, then each field from the request struct
        let mut call_args = vec!["ctx: ctx".to_string()];
        call_args.extend(method.params.iter().map(|p| {
            let name = p.name.to_case(Case::Camel);
            format!("{name}: req.{name}")
        }));

        if method.return_type.is_some() {
            writeln!(
                out,
                "            let result = try await handler.{case_name}({})",
                call_args.join(", ")
            )
            .unwrap();
            writeln!(
                out,
                "            return Frame(tag: tag, msg: .{case_name}(R{pascal}(value: result)))"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "            try await handler.{case_name}({})",
                call_args.join(", ")
            )
            .unwrap();
            writeln!(
                out,
                "            return Frame(tag: tag, msg: .{case_name}(R{pascal}()))"
            )
            .unwrap();
        }
    }

    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }} catch let error as JetStreamError {{").unwrap();
    writeln!(out, "        return Frame(tag: tag, msg: .error(error))")
        .unwrap();
    writeln!(out, "    }} catch {{").unwrap();
    writeln!(
        out,
        "        let jsErr = JetStreamError(inner: ErrorInner(message: \"\\(error)\"))"
    )
    .unwrap();
    writeln!(out, "        return Frame(tag: tag, msg: .error(jsErr))")
        .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
}
