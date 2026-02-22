// r[impl jetstream.codegen.service.ts]

use std::collections::BTreeSet;
use std::fmt::Write;

use convert_case::{Case, Casing};
use typeshare_core::rust_types::{RustType, SpecialRustType};

use crate::service_parser::{MethodDef, ServiceDef};
use crate::ts_backend::{rust_type_to_ts, rust_type_to_ts_codec, TsConfig};

/// Collect all custom (non-primitive) type names referenced by a service's methods.
/// These are `RustType::Simple` or `RustType::Generic` types that live in the
/// sibling types file and need to be imported.
fn collect_custom_types(service: &ServiceDef) -> BTreeSet<String> {
    let mut types = BTreeSet::new();
    for method in &service.methods {
        for p in &method.params {
            collect_custom_type_names(&p.ty, &mut types);
        }
        if let Some(ret) = &method.return_type {
            collect_custom_type_names(ret, &mut types);
        }
    }
    types
}

/// Type names that are scalar primitives handled by the wireformat package,
/// not custom types from the sibling types module.
const PRIMITIVE_SCALARS: &[&str] = &["u128", "i128"];

fn collect_custom_type_names(ty: &RustType, types: &mut BTreeSet<String>) {
    match ty {
        RustType::Simple { id } => {
            if !PRIMITIVE_SCALARS.contains(&id.as_str()) {
                types.insert(id.clone());
            }
        }
        RustType::Generic { id, parameters } => {
            types.insert(id.clone());
            for p in parameters {
                collect_custom_type_names(p, types);
            }
        }
        RustType::Special(special) => match special {
            SpecialRustType::Vec(inner)
            | SpecialRustType::Slice(inner)
            | SpecialRustType::Array(inner, _)
            | SpecialRustType::Option(inner) => {
                collect_custom_type_names(inner, types);
            }
            SpecialRustType::HashMap(k, v) => {
                collect_custom_type_names(k, types);
                collect_custom_type_names(v, types);
            }
            _ => {}
        },
    }
}

/// Collect all codec names used by a service's methods so we can import them.
fn collect_codecs(service: &ServiceDef) -> BTreeSet<String> {
    let mut codecs = BTreeSet::new();
    for method in &service.methods {
        for p in &method.params {
            collect_codec_names(&p.ty, &mut codecs);
        }
        if let Some(ret) = &method.return_type {
            collect_codec_names(ret, &mut codecs);
        }
    }
    codecs
}

fn collect_codec_names(ty: &RustType, codecs: &mut BTreeSet<String>) {
    // Handle u128/i128 which typeshare_core doesn't have as SpecialRustType variants
    if let RustType::Simple { id } = ty {
        if id == "u128" {
            codecs.insert("u128Codec".into());
            return;
        }
        if id == "i128" {
            codecs.insert("i128Codec".into());
            return;
        }
    }
    if let RustType::Special(special) = ty {
        match special {
            SpecialRustType::U8 | SpecialRustType::I8 => {
                codecs.insert("u8Codec".into());
            }
            SpecialRustType::U16 => {
                codecs.insert("u16Codec".into());
            }
            SpecialRustType::U32 => {
                codecs.insert("u32Codec".into());
            }
            SpecialRustType::U64 | SpecialRustType::USize => {
                codecs.insert("u64Codec".into());
            }
            SpecialRustType::I16 => {
                codecs.insert("i16Codec".into());
            }
            SpecialRustType::I32 => {
                codecs.insert("i32Codec".into());
            }
            SpecialRustType::I64 | SpecialRustType::ISize => {
                codecs.insert("i64Codec".into());
            }
            SpecialRustType::F32 => {
                codecs.insert("f32Codec".into());
            }
            SpecialRustType::F64 => {
                codecs.insert("f64Codec".into());
            }
            SpecialRustType::Bool => {
                codecs.insert("boolCodec".into());
            }
            SpecialRustType::String | SpecialRustType::Char => {
                codecs.insert("stringCodec".into());
            }
            SpecialRustType::Unit => {
                codecs.insert("unitCodec".into());
            }
            SpecialRustType::Vec(inner) | SpecialRustType::Slice(inner) => {
                codecs.insert("vecCodec".into());
                collect_codec_names(inner, codecs);
            }
            SpecialRustType::Array(inner, _) => {
                codecs.insert("vecCodec".into());
                collect_codec_names(inner, codecs);
            }
            SpecialRustType::Option(inner) => {
                codecs.insert("optionCodec".into());
                collect_codec_names(inner, codecs);
            }
            SpecialRustType::HashMap(k, v) => {
                codecs.insert("mapCodec".into());
                collect_codec_names(k, codecs);
                collect_codec_names(v, codecs);
            }
            _ => {}
        }
    }
}

/// r[jetstream.rpc.ts.message-ids]
/// Generate TypeScript RPC code from a ServiceDef.
///
/// This generates:
/// - Request/response interfaces and codecs
/// - Tmessage/Rmessage discriminated unions with Framer impl
/// - Client class with async methods
/// - Handler interface
pub fn generate_ts_rpc(
    service: &ServiceDef,
    config: &TsConfig,
    types_module: &str,
) -> String {
    let mut out = String::from("// @ts-nocheck — generated file\n");

    // Collect used codecs for import
    let codecs = collect_codecs(service);
    let codec_imports: Vec<&str> = codecs.iter().map(|s| s.as_str()).collect();

    // Imports — consolidated barrel imports
    let mut wf_imports = vec![
        "BinaryReader",
        "BinaryWriter",
        "jetStreamErrorCodec",
        "JetStreamError",
    ];
    wf_imports.extend(codec_imports);
    writeln!(
        out,
        "import {{ {} }} from '{}';",
        wf_imports.join(", "),
        config.import_path
    )
    .unwrap();
    writeln!(
        out,
        "import type {{ WireFormat }} from '{}';",
        config.import_path
    )
    .unwrap();
    writeln!(
        out,
        "import {{ Mux, negotiateVersion }} from '{}';",
        config.rpc_import_path
    )
    .unwrap();
    writeln!(
        out,
        "import type {{ Framer, FramerCodec, Context, NegotiatedVersion }} from '{}';",
        config.rpc_import_path
    )
    .unwrap();

    // Import custom types and their codecs from the sibling types file
    let custom_types = collect_custom_types(service);
    if !custom_types.is_empty() {
        let type_names: Vec<&str> =
            custom_types.iter().map(|s| s.as_str()).collect();
        let codec_names: Vec<String> = custom_types
            .iter()
            .map(|s| format!("{}Codec", s.to_case(Case::Camel)))
            .collect();
        writeln!(
            out,
            "import type {{ {} }} from './{types_module}';",
            type_names.join(", "),
        )
        .unwrap();
        writeln!(
            out,
            "import {{ {} }} from './{types_module}';",
            codec_names.join(", "),
        )
        .unwrap();
    }

    // Extra RPC imports
    for extra in &config.rpc_extra_imports {
        writeln!(out, "{extra}").unwrap();
    }
    writeln!(out).unwrap();

    let name = &service.name;

    // Message ID constants
    writeln!(out, "const MESSAGE_ID_START = 102;").unwrap();
    writeln!(out, "const RERROR = 5;").unwrap();
    writeln!(out).unwrap();

    for method in &service.methods {
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "const T{upper}: number = {};", method.request_id)
            .unwrap();
        writeln!(out, "const R{upper}: number = {};", method.response_id)
            .unwrap();
    }
    writeln!(out).unwrap();

    // Request/response types and codecs
    for method in &service.methods {
        generate_ts_message_types(&mut out, method);
    }

    // Tmessage union
    generate_ts_tmessage(&mut out, &service.methods);
    writeln!(out).unwrap();

    // Rmessage union
    generate_ts_rmessage(&mut out, &service.methods);
    writeln!(out).unwrap();

    // r[jetstream.rpc.ts.framer]
    // FramerCodec objects (used by ServerCodec / handler side)
    generate_ts_tmessage_framer(&mut out, &service.methods);
    writeln!(out).unwrap();
    generate_ts_rmessage_framer(&mut out, &service.methods);
    writeln!(out).unwrap();

    // Framer wrapper classes (implement Framer interface for use with Mux)
    generate_ts_tmessage_framer_class(&mut out, &service.methods);
    writeln!(out).unwrap();
    generate_ts_rmessage_framer_class(&mut out, &service.methods);
    writeln!(out).unwrap();

    // PROTOCOL_VERSION constant — matches the Rust macro format:
    // rs.jetstream.proto/{name}/{major}.{minor}.{patch}-{digest_prefix}
    let name_lower = name.to_lowercase();
    let version = if service.version.is_empty() {
        "0.0.0"
    } else {
        &service.version
    };
    let digest = &service.digest;
    writeln!(
        out,
        "export const PROTOCOL_NAME = 'rs.jetstream.proto/{name_lower}';",
    )
    .unwrap();
    writeln!(
        out,
        "export const PROTOCOL_VERSION = 'rs.jetstream.proto/{name_lower}/{version}+{digest}';",
    )
    .unwrap();
    writeln!(out).unwrap();

    // Client class (uses library Mux)
    generate_ts_client(&mut out, name, &service.methods);
    writeln!(out).unwrap();

    // Handler interface
    generate_ts_handler(&mut out, name, &service.methods);

    out
}

fn generate_ts_message_types(out: &mut String, method: &MethodDef) {
    let pascal = method.name.to_case(Case::Pascal);
    let camel = method.name.to_case(Case::Camel);

    // Request interface + codec
    writeln!(out, "export interface T{pascal} {{").unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        let ts_type = rust_type_to_ts(&p.ty);
        writeln!(out, "  {field_name}: {ts_type};").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "export const t{camel}Codec: WireFormat<T{pascal}> = {{"
    )
    .unwrap();

    // byteSize
    writeln!(out, "  byteSize(value: T{pascal}): number {{").unwrap();
    if method.params.is_empty() {
        writeln!(out, "    return 0;").unwrap();
    } else {
        let parts: Vec<String> = method
            .params
            .iter()
            .map(|p| {
                let field_name = p.name.to_case(Case::Camel);
                let codec = rust_type_to_ts_codec(&p.ty);
                format!("{codec}.byteSize(value.{field_name})")
            })
            .collect();
        writeln!(out, "    return {};", parts.join(" + ")).unwrap();
    }
    writeln!(out, "  }},").unwrap();

    // encode
    writeln!(
        out,
        "  encode(value: T{pascal}, writer: BinaryWriter): void {{"
    )
    .unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        let codec = rust_type_to_ts_codec(&p.ty);
        writeln!(out, "    {codec}.encode(value.{field_name}, writer);")
            .unwrap();
    }
    writeln!(out, "  }},").unwrap();

    // decode
    writeln!(out, "  decode(reader: BinaryReader): T{pascal} {{").unwrap();
    for p in &method.params {
        let field_name = p.name.to_case(Case::Camel);
        let codec = rust_type_to_ts_codec(&p.ty);
        writeln!(out, "    const {field_name} = {codec}.decode(reader);")
            .unwrap();
    }
    let field_list: Vec<String> = method
        .params
        .iter()
        .map(|p| p.name.to_case(Case::Camel))
        .collect();
    writeln!(out, "    return {{ {} }};", field_list.join(", ")).unwrap();
    writeln!(out, "  }},").unwrap();
    writeln!(out, "}};").unwrap();
    writeln!(out).unwrap();

    // Response interface + codec
    let ret_type = method
        .return_type
        .as_ref()
        .map(rust_type_to_ts)
        .unwrap_or_else(|| "undefined".into());
    let ret_codec = method
        .return_type
        .as_ref()
        .map(rust_type_to_ts_codec)
        .unwrap_or_else(|| "unitCodec".into());

    writeln!(out, "export interface R{pascal} {{").unwrap();
    if method.return_type.is_some() {
        writeln!(out, "  value: {ret_type};").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "export const r{camel}Codec: WireFormat<R{pascal}> = {{"
    )
    .unwrap();
    writeln!(out, "  byteSize(value: R{pascal}): number {{").unwrap();
    if method.return_type.is_some() {
        writeln!(out, "    return {ret_codec}.byteSize(value.value);").unwrap();
    } else {
        writeln!(out, "    return 0;").unwrap();
    }
    writeln!(out, "  }},").unwrap();
    writeln!(
        out,
        "  encode(value: R{pascal}, writer: BinaryWriter): void {{"
    )
    .unwrap();
    if method.return_type.is_some() {
        writeln!(out, "    {ret_codec}.encode(value.value, writer);").unwrap();
    }
    writeln!(out, "  }},").unwrap();
    writeln!(out, "  decode(reader: BinaryReader): R{pascal} {{").unwrap();
    if method.return_type.is_some() {
        writeln!(out, "    return {{ value: {ret_codec}.decode(reader) }};")
            .unwrap();
    } else {
        writeln!(out, "    return {{}};").unwrap();
    }
    writeln!(out, "  }},").unwrap();
    writeln!(out, "}};").unwrap();
    writeln!(out).unwrap();
}

fn generate_ts_tmessage(out: &mut String, methods: &[MethodDef]) {
    writeln!(out, "export type Tmessage =").unwrap();
    for (i, method) in methods.iter().enumerate() {
        let pascal = method.name.to_case(Case::Pascal);
        let sep = if i < methods.len() - 1 { "" } else { ";" };
        writeln!(out, "  | {{ type: '{pascal}'; msg: T{pascal} }}{sep}")
            .unwrap();
    }
}

fn generate_ts_rmessage(out: &mut String, methods: &[MethodDef]) {
    writeln!(out, "export type Rmessage =").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        writeln!(out, "  | {{ type: '{pascal}'; msg: R{pascal} }}").unwrap();
    }
    writeln!(out, "  | {{ type: 'Error'; msg: JetStreamError }};").unwrap();
}

fn generate_ts_tmessage_framer(out: &mut String, methods: &[MethodDef]) {
    writeln!(
        out,
        "export const tmessageFramer: FramerCodec<Tmessage> = {{"
    )
    .unwrap();

    // messageType
    writeln!(out, "  messageType(msg: Tmessage): number {{").unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "      case '{pascal}': return T{upper};").unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // byteSize
    writeln!(out, "  byteSize(msg: Tmessage): number {{").unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case '{pascal}': return t{camel}Codec.byteSize(msg.msg);"
        )
        .unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // encode
    writeln!(
        out,
        "  encode(msg: Tmessage, writer: BinaryWriter): void {{"
    )
    .unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case '{pascal}': t{camel}Codec.encode(msg.msg, writer); break;"
        )
        .unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // decode
    writeln!(
        out,
        "  decode(reader: BinaryReader, type: number): Tmessage {{"
    )
    .unwrap();
    writeln!(out, "    switch (type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let upper = method.name.to_case(Case::UpperSnake);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case T{upper}: return {{ type: '{pascal}', msg: t{camel}Codec.decode(reader) }};"
        )
        .unwrap();
    }
    writeln!(
        out,
        "      default: throw new Error(`unknown Tmessage type: ${{type}}`);"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    writeln!(out, "}};").unwrap();
}

fn generate_ts_rmessage_framer(out: &mut String, methods: &[MethodDef]) {
    writeln!(
        out,
        "export const rmessageFramer: FramerCodec<Rmessage> = {{"
    )
    .unwrap();

    // messageType
    writeln!(out, "  messageType(msg: Rmessage): number {{").unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let upper = method.name.to_case(Case::UpperSnake);
        writeln!(out, "      case '{pascal}': return R{upper};").unwrap();
    }
    writeln!(out, "      case 'Error': return RERROR;").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // byteSize
    writeln!(out, "  byteSize(msg: Rmessage): number {{").unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case '{pascal}': return r{camel}Codec.byteSize(msg.msg);"
        )
        .unwrap();
    }
    writeln!(
        out,
        "      case 'Error': return jetStreamErrorCodec.byteSize(msg.msg);"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // encode
    writeln!(
        out,
        "  encode(msg: Rmessage, writer: BinaryWriter): void {{"
    )
    .unwrap();
    writeln!(out, "    switch (msg.type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case '{pascal}': r{camel}Codec.encode(msg.msg, writer); break;"
        )
        .unwrap();
    }
    writeln!(
        out,
        "      case 'Error': jetStreamErrorCodec.encode(msg.msg, writer); break;"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // r[jetstream.rpc.ts.error-frame]
    // decode
    writeln!(
        out,
        "  decode(reader: BinaryReader, type: number): Rmessage {{"
    )
    .unwrap();
    writeln!(out, "    switch (type) {{").unwrap();
    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let upper = method.name.to_case(Case::UpperSnake);
        let camel = method.name.to_case(Case::Camel);
        writeln!(
            out,
            "      case R{upper}: return {{ type: '{pascal}', msg: r{camel}Codec.decode(reader) }};"
        )
        .unwrap();
    }
    writeln!(
        out,
        "      case RERROR: return {{ type: 'Error', msg: jetStreamErrorCodec.decode(reader) }};"
    )
    .unwrap();
    writeln!(
        out,
        "      default: throw new Error(`unknown Rmessage type: ${{type}}`);"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    writeln!(out, "}};").unwrap();
}

/// Generate a TmessageFramer class that implements the Framer interface,
/// wrapping a Tmessage discriminated union and delegating to tmessageFramer.
fn generate_ts_tmessage_framer_class(out: &mut String, _methods: &[MethodDef]) {
    writeln!(out, "export class TmessageFramer implements Framer {{").unwrap();
    writeln!(out, "  readonly inner: Tmessage;").unwrap();
    writeln!(
        out,
        "  constructor(inner: Tmessage) {{ this.inner = inner; }}"
    )
    .unwrap();
    writeln!(out, "  messageType(): number {{ return tmessageFramer.messageType(this.inner); }}").unwrap();
    writeln!(out, "  byteSize(): number {{ return tmessageFramer.byteSize(this.inner); }}").unwrap();
    writeln!(out, "  encode(writer: BinaryWriter): void {{ tmessageFramer.encode(this.inner, writer); }}").unwrap();
    writeln!(out, "}}").unwrap();
}

/// Generate an RmessageFramer class that implements the Framer interface,
/// wrapping an Rmessage discriminated union and delegating to rmessageFramer.
/// Also generates the static decode function for use with Mux/Transport.
fn generate_ts_rmessage_framer_class(out: &mut String, _methods: &[MethodDef]) {
    writeln!(out, "export class RmessageFramer implements Framer {{").unwrap();
    writeln!(out, "  readonly inner: Rmessage;").unwrap();
    writeln!(
        out,
        "  constructor(inner: Rmessage) {{ this.inner = inner; }}"
    )
    .unwrap();
    writeln!(out, "  messageType(): number {{ return rmessageFramer.messageType(this.inner); }}").unwrap();
    writeln!(out, "  byteSize(): number {{ return rmessageFramer.byteSize(this.inner); }}").unwrap();
    writeln!(out, "  encode(writer: BinaryWriter): void {{ rmessageFramer.encode(this.inner, writer); }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "export function rmessageDecode(reader: BinaryReader, type: number): RmessageFramer {{").unwrap();
    writeln!(
        out,
        "  return new RmessageFramer(rmessageFramer.decode(reader, type));"
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
}

// r[impl jetstream.rpc.ts.client]
fn generate_ts_client(
    out: &mut String,
    service_name: &str,
    methods: &[MethodDef],
) {
    let client_name = format!("{service_name}Client");

    writeln!(out, "export class {client_name} {{").unwrap();
    writeln!(out, "  private mux: Mux<TmessageFramer, RmessageFramer>;")
        .unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "  constructor(mux: Mux<TmessageFramer, RmessageFramer>) {{"
    )
    .unwrap();
    writeln!(out, "    this.mux = mux;").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out).unwrap();

    // r[impl jetstream.version.framer.client-handshake]
    // Static factory that negotiates version on a raw bidi stream before creating Mux + client
    writeln!(out, "  static async negotiate(").unwrap();
    writeln!(out, "    readable: ReadableStream<Uint8Array>,").unwrap();
    writeln!(out, "    writable: WritableStream<Uint8Array>,").unwrap();
    writeln!(out, "    msize: number = 65536,").unwrap();
    writeln!(out, "  ): Promise<NegotiatedVersion> {{").unwrap();
    writeln!(out, "    return negotiateVersion(readable, writable, PROTOCOL_VERSION, msize);").unwrap();
    writeln!(out, "  }}").unwrap();

    for method in methods {
        let method_name = method.name.to_case(Case::Camel);
        let pascal = method.name.to_case(Case::Pascal);

        let params: Vec<String> = method
            .params
            .iter()
            .map(|p| {
                let name = p.name.to_case(Case::Camel);
                let ts_type = rust_type_to_ts(&p.ty);
                format!("{name}: {ts_type}")
            })
            .collect();

        let ret_type = method
            .return_type
            .as_ref()
            .map(rust_type_to_ts)
            .unwrap_or_else(|| "void".into());

        writeln!(out).unwrap();
        writeln!(
            out,
            "  async {method_name}({}): Promise<{ret_type}> {{",
            params.join(", ")
        )
        .unwrap();

        let args: Vec<String> = method
            .params
            .iter()
            .map(|p| p.name.to_case(Case::Camel))
            .collect();
        writeln!(
            out,
            "    const req = new TmessageFramer({{ type: '{pascal}', msg: {{ {} }} }});",
            args.join(", ")
        )
        .unwrap();
        writeln!(out, "    const res = await this.mux.rpc(req);").unwrap();
        writeln!(out, "    if (res.msg.inner.type === 'Error') {{").unwrap();
        writeln!(out, "      throw res.msg.inner.msg;").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "    if (res.msg.inner.type !== '{pascal}') {{").unwrap();
        writeln!(
            out,
            "      throw new Error(`unexpected response type: ${{res.msg.inner.type}}`);"
        )
        .unwrap();
        writeln!(out, "    }}").unwrap();
        if method.return_type.is_some() {
            writeln!(out, "    return res.msg.inner.msg.value;").unwrap();
        }
        writeln!(out, "  }}").unwrap();
    }

    writeln!(out, "}}").unwrap();
}

// r[impl jetstream.rpc.ts.handler]
fn generate_ts_handler(
    out: &mut String,
    service_name: &str,
    methods: &[MethodDef],
) {
    let handler_name = format!("{service_name}Handler");

    // Handler interface with ctx: Context as first parameter
    writeln!(out, "export interface {handler_name} {{").unwrap();
    for method in methods {
        let method_name = method.name.to_case(Case::Camel);

        let mut params = vec!["ctx: Context".to_string()];
        for p in &method.params {
            let name = p.name.to_case(Case::Camel);
            let ts_type = rust_type_to_ts(&p.ty);
            params.push(format!("{name}: {ts_type}"));
        }

        let ret_type = method
            .return_type
            .as_ref()
            .map(rust_type_to_ts)
            .unwrap_or_else(|| "void".into());

        writeln!(
            out,
            "  {method_name}({}): Promise<{ret_type}>;",
            params.join(", ")
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // r[impl jetstream.rpc.ts.handler.dispatch]
    // Dispatch function: pattern-match on Tmessage type, call handler, wrap in Rmessage
    let dispatch_name = format!("dispatch{service_name}");
    writeln!(out, "export async function {dispatch_name}(").unwrap();
    writeln!(out, "  handler: {handler_name},").unwrap();
    writeln!(out, "  ctx: Context,").unwrap();
    writeln!(out, "  frame: {{ tag: number; msg: Tmessage }},").unwrap();
    writeln!(out, "): Promise<{{ tag: number; msg: Rmessage }}> {{").unwrap();
    writeln!(out, "  try {{").unwrap();
    writeln!(out, "    switch (frame.msg.type) {{").unwrap();

    for method in methods {
        let pascal = method.name.to_case(Case::Pascal);
        let method_name = method.name.to_case(Case::Camel);

        writeln!(out, "      case '{pascal}': {{").unwrap();

        // Build call arguments: ctx, then each field from frame.msg.msg
        let args: Vec<String> = std::iter::once("ctx".to_string())
            .chain(method.params.iter().map(|p| {
                format!("frame.msg.msg.{}", p.name.to_case(Case::Camel))
            }))
            .collect();
        let call_args = args.join(", ");

        if method.return_type.is_some() {
            writeln!(
                out,
                "        const result = await handler.{method_name}({call_args});"
            )
            .unwrap();
            writeln!(
                out,
                "        return {{ tag: frame.tag, msg: {{ type: '{pascal}', msg: {{ value: result }} }} }};"
            )
            .unwrap();
        } else {
            writeln!(out, "        await handler.{method_name}({call_args});")
                .unwrap();
            writeln!(
                out,
                "        return {{ tag: frame.tag, msg: {{ type: '{pascal}', msg: {{}} }} }};"
            )
            .unwrap();
        }
        writeln!(out, "      }}").unwrap();
    }

    writeln!(out, "    }}").unwrap();
    // Exhaustive check — should never be reached with correct types
    writeln!(
        out,
        "    throw new Error(`unknown message type: ${{(frame.msg as any).type}}`);"
    )
    .unwrap();
    writeln!(out, "  }} catch (err) {{").unwrap();
    writeln!(out, "    if (err instanceof JetStreamError) {{").unwrap();
    writeln!(
        out,
        "      return {{ tag: frame.tag, msg: {{ type: 'Error', msg: err }} }};"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    const jsErr = new JetStreamError(").unwrap();
    writeln!(
        out,
        "      {{ message: String(err), code: null, help: null, url: null }},"
    )
    .unwrap();
    writeln!(out, "      {{ internTable: [''], frames: [] }},").unwrap();
    writeln!(out, "    );").unwrap();
    writeln!(
        out,
        "    return {{ tag: frame.tag, msg: {{ type: 'Error', msg: jsErr }} }};"
    )
    .unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out, "}}").unwrap();
}
