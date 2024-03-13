use std::{io::Write, ops::Add, path::PathBuf};

use slog::{slog_o, Drain, Logger};

use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

// get module colour hashes the module name
// and attempts to return a uniqe color as far as ansi colors go
fn get_module_colour(module: &str) -> Color {
    // crc16 is a good hash for this
    let hash = crc16::State::<crc16::XMODEM>::calculate(module.as_bytes());
    let hash = hash.add(5);
    let color = match hash % 6 {
        0 => Color::Red,
        1 => Color::Green,
        2 => Color::Yellow,
        3 => Color::Blue,
        4 => Color::Magenta,
        5 => Color::Cyan,
        _ => Color::White,
    };
    color
}

pub fn setup_logging() -> Logger {
    let x = drain();

    slog::Logger::root(x, slog_o!())
}

pub fn drain() -> slog::Fuse<
    slog_term::FullFormat<slog_term::PlainSyncDecorator<std::io::Stdout>>,
> {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let ff = slog_term::FullFormat::new(plain);

    let x = ff
        .use_custom_header_print(|_f, _t, r, _x| {
            // print format is: dev.branch/{module} {level} {msg}
            // module should be cleaned by :: -> /
            // level should be colored use termcolor
            let module = r.module().replace("::", "/");
            let level = match r.level() {
                slog::Level::Critical => termcolor::Color::Red,
                slog::Level::Error => termcolor::Color::Red,
                slog::Level::Warning => termcolor::Color::Yellow,
                slog::Level::Info => termcolor::Color::Green,
                slog::Level::Debug => termcolor::Color::Blue,
                slog::Level::Trace => termcolor::Color::Cyan,
            };
            let cargo_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            // drop last component

            let mut location_buffer = PathBuf::from_iter(
                cargo_path
                    .components()
                    .take(cargo_path.components().count() - 1),
            );
            location_buffer.push(r.file());
            let loc = location_buffer.to_str().unwrap();
            let bufwtr = BufferWriter::stderr(ColorChoice::Always);
            let mut buffer = bufwtr.buffer();
            let module_color = get_module_colour(&module);
            buffer.set_color(ColorSpec::new().set_fg(Some(module_color)))?;
            let _ = write!(buffer, "dev.branch.jetstream/{} ", module,);
            buffer.reset()?;
            buffer.set_color(
                ColorSpec::new()
                    .set_dimmed(true)
                    .set_underline(true)
                    .set_fg(Some(Color::White)),
            )?;
            let _ = write!(buffer, "{}:{}", loc.to_string(), r.location().line);
            buffer.reset()?;
            buffer.set_color(
                ColorSpec::new().set_fg(Some(level)).set_intense(true),
            )?;
            let _ = write!(buffer, " {}", r.level());
            buffer.reset()?;
            let _ = write!(buffer, " {}", r.msg());
            let _ = bufwtr.print(&buffer);
            std::result::Result::Ok(true)
        })
        .build()
        .fuse();
    x
}
