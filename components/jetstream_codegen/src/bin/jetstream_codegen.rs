use argh::FromArgs;
use cargo_metadata::MetadataCommand;
use std::path::PathBuf;

use jetstream_codegen::parser::parse_file;
use jetstream_codegen::service_parser::parse_services_from_file;
use jetstream_codegen::swift_backend::{generate_swift_file, SwiftConfig};
use jetstream_codegen::swift_rpc_backend::generate_swift_rpc;
use jetstream_codegen::ts_backend::{generate_ts_file, TsConfig};
use jetstream_codegen::ts_rpc_backend::generate_ts_rpc;

/// JetStream code generator: transforms Rust types annotated with
/// #[derive(JetStreamWireFormat)] and #[service] traits into TypeScript
/// and Swift WireFormat codec implementations.
#[derive(FromArgs)]
struct Args {
    /// input Rust source file
    #[argh(option)]
    input: PathBuf,

    /// output directory for TypeScript files
    #[argh(option)]
    ts_out: Option<PathBuf>,

    /// output directory for Swift files
    #[argh(option)]
    swift_out: Option<PathBuf>,

    /// import path for TypeScript wireformat module
    #[argh(option, default = "String::from(\"@sevki/jetstream-wireformat\")")]
    ts_import_path: String,

    /// import path for TypeScript RPC module
    #[argh(option, default = "String::from(\"@sevki/jetstream-rpc\")")]
    ts_rpc_import_path: String,

    /// import module name for Swift wireformat
    #[argh(option, default = "String::from(\"JetStreamWireFormat\")")]
    swift_module: String,
}

fn main() {
    let args: Args = argh::from_env();

    let source = match std::fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {e}", args.input.display());
            std::process::exit(1);
        }
    };

    // Resolve package version from cargo metadata
    let input_abs = std::fs::canonicalize(&args.input).unwrap_or_else(|e| {
        eprintln!("error resolving {}: {e}", args.input.display());
        std::process::exit(1);
    });
    let pkg_version = find_package_version(&input_abs);

    let items = parse_file(&source);
    let services = parse_services_from_file(&source, &pkg_version);

    let stem = args
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // TypeScript output
    if let Some(ts_dir) = &args.ts_out {
        std::fs::create_dir_all(ts_dir).unwrap();

        let ts_config = TsConfig {
            import_path: args.ts_import_path.clone(),
            rpc_import_path: args.ts_rpc_import_path.clone(),
        };

        if !items.is_empty() {
            let ts_types = generate_ts_file(&items, &ts_config);
            let ts_path = ts_dir.join(format!("{stem}.ts"));
            std::fs::write(&ts_path, ts_types).unwrap();
            eprintln!("wrote {}", ts_path.display());
        }

        for service in &services {
            let ts_rpc = generate_ts_rpc(service, &ts_config);
            let rpc_path =
                ts_dir.join(format!("{}_rpc.ts", service.name.to_lowercase()));
            std::fs::write(&rpc_path, ts_rpc).unwrap();
            eprintln!("wrote {}", rpc_path.display());
        }
    }

    // Swift output
    if let Some(swift_dir) = &args.swift_out {
        std::fs::create_dir_all(swift_dir).unwrap();

        let swift_config = SwiftConfig {
            wireformat_module: args.swift_module.clone(),
        };

        if !items.is_empty() {
            let swift_types = generate_swift_file(&items, &swift_config);
            let swift_path = swift_dir.join(format!("{stem}.swift"));
            std::fs::write(&swift_path, swift_types).unwrap();
            eprintln!("wrote {}", swift_path.display());
        }

        for service in &services {
            let swift_rpc = generate_swift_rpc(service, &swift_config);
            let rpc_path = swift_dir.join(format!("{}Rpc.swift", service.name));
            std::fs::write(&rpc_path, swift_rpc).unwrap();
            eprintln!("wrote {}", rpc_path.display());
        }
    }

    if args.ts_out.is_none() && args.swift_out.is_none() {
        eprintln!(
            "no output directory specified; use --ts-out and/or --swift-out"
        );
        std::process::exit(1);
    }
}

/// Walk up from a source file path to find the owning Cargo package and return
/// its semver version string (e.g. "15.0.0").
fn find_package_version(input_path: &std::path::Path) -> String {
    // Find the manifest directory by walking up from the input file
    let mut manifest_dir = input_path.parent();
    let manifest_path = loop {
        match manifest_dir {
            Some(dir) => {
                let candidate = dir.join("Cargo.toml");
                if candidate.exists() {
                    break candidate;
                }
                manifest_dir = dir.parent();
            }
            None => {
                eprintln!("no Cargo.toml found above {}", input_path.display());
                std::process::exit(1);
            }
        }
    };

    let metadata = MetadataCommand::new()
        .manifest_path(&manifest_path)
        .no_deps()
        .exec()
        .unwrap_or_else(|e| {
            eprintln!("cargo metadata failed: {e}");
            std::process::exit(1);
        });

    // Find the package whose manifest_path matches
    for pkg in &metadata.packages {
        if pkg.manifest_path == manifest_path {
            return pkg.version.to_string();
        }
    }

    // Fallback: if there's exactly one package, use it
    if metadata.packages.len() == 1 {
        return metadata.packages[0].version.to_string();
    }

    eprintln!(
        "could not determine package version from {}",
        manifest_path.display()
    );
    std::process::exit(1);
}
