//! Backtrace related types and functions.

use {
    jetstream_macros::JetStreamWireFormat,
    string_interner::{
        backend::BucketBackend, symbol::SymbolU16, StringInterner, Symbol,
    },
    tracing::Level,
};

#[cfg(all(feature = "miette", feature = "source-info"))]
use source_map::{
    encodings::Utf8, FileSystem, LineColumnPosition, MapFileStore, NoPathMap,
};

use tracing_error::SpanTrace;

#[cfg(feature = "miette")]
/// Severity level derived from tracing Level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Severity {
    Warning,
    Error,
}

#[cfg(feature = "miette")]
impl From<Severity> for miette::Severity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Warning => miette::Severity::Warning,
            Severity::Error => miette::Severity::Error,
        }
    }
}

// r[impl jetstream.error.v2.backtrace]
// r[impl jetstream.error.v2.wireformat.intern-table]
#[derive(Clone, Debug, Default, JetStreamWireFormat)]
/// A backtrace that can be understood by <https://crashdu.mp>.
pub struct Backtrace {
    intern_table: Vec<String>,
    pub(crate) frames: Vec<Frame>,
}

// r[impl jetstream.error.v2.backtrace.field-pair]
#[derive(Clone, Debug, JetStreamWireFormat)]
pub(crate) struct FieldPair {
    key: u16,
    value: u16,
}

// r[impl jetstream.error.v2.backtrace.frame]
// r[impl jetstream.error.v2.wireformat.frame]
// r[impl jetstream.error.v2.source-info]
#[derive(Clone, Debug, JetStreamWireFormat)]
pub(crate) struct Frame {
    msg: String,
    name: u16,
    target: u16,
    module: u16,
    file: u16,
    line: u16,
    fields: Vec<FieldPair>,
    #[jetstream(with(crate::coding::LevelCodec))]
    level: Level,
}

#[cfg(feature = "miette")]
#[derive(Clone, Debug)]
pub(crate) struct TraceDiagnostic {
    msg: String,
    level: Level,
    source: Option<miette::NamedSource<String>>,
    label_offset: usize,
    label_len: usize,
    fields: Vec<String>,
}

#[cfg(not(feature = "miette"))]
#[derive(Clone, Debug)]
pub(crate) struct TraceDiagnostic {
    pub(crate) msg: String,
    #[allow(dead_code)]
    pub(crate) level: Level,
    pub(crate) fields: Vec<String>,
    pub(crate) file: Option<String>,
    pub(crate) line: u16,
}

impl std::fmt::Display for TraceDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for TraceDiagnostic {}

#[cfg(feature = "miette")]
impl miette::Diagnostic for TraceDiagnostic {
    fn severity(&self) -> Option<miette::Severity> {
        match self.level {
            Level::DEBUG | Level::TRACE | Level::INFO => None,
            Level::WARN => Some(miette::Severity::Warning),
            Level::ERROR => Some(miette::Severity::Error),
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.source.as_ref().map(|s| s as &dyn miette::SourceCode)
    }

    fn labels(
        &self,
    ) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        if self.source.is_some() {
            let label_text =
                format!("{}({})", self.msg.clone(), self.fields.join(", "));
            Some(Box::new(std::iter::once(miette::LabeledSpan::at(
                self.label_offset..self.label_offset + self.label_len,
                label_text,
            ))))
        } else {
            None
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            msg: String::new(),
            name: 0,
            target: 0,
            module: 0,
            file: 0,
            line: 0,
            fields: Vec::new(),
            level: Level::ERROR,
        }
    }
}

// r[impl jetstream.error.v2.reporting.source-resolution]
/// Try to read a source file, walking up parent directories to handle
/// workspace-relative paths from `file!()`.
#[cfg(all(feature = "miette", feature = "source-info"))]
fn read_source_file(path: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    // If absolute, try directly
    if p.is_absolute() {
        return std::fs::read_to_string(p).ok();
    }
    // Try relative to CWD, then walk up parents
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..10 {
        let candidate = dir.join(p);
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            return Some(content);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

impl Backtrace {
    fn resolve_fields(&self, frame: &Frame) -> Vec<String> {
        frame
            .fields
            .iter()
            .filter_map(|fp| {
                let k = self.intern_table.get(fp.key as usize)?;
                let v = self.intern_table.get(fp.value as usize)?;
                Some(format!("{}: {}", k, v))
            })
            .collect()
    }

    /// Returns resolved diagnostics for each frame (miette + source-info).
    #[cfg(all(feature = "miette", feature = "source-info"))]
    pub(crate) fn diagnostics(&self) -> Vec<TraceDiagnostic> {
        let mut fs = MapFileStore::<NoPathMap>::default();
        let mut source_cache: std::collections::HashMap<
            u16,
            Option<(source_map::SourceId, String)>,
        > = std::collections::HashMap::new();

        self.frames
            .iter()
            .map(|frame| {
                let resolved = source_cache
                    .entry(frame.file)
                    .or_insert_with(|| {
                        let path =
                            self.intern_table.get(frame.file as usize)?;
                        let content = read_source_file(path)?;
                        let source_id =
                            fs.new_source_id(path.into(), content.clone());
                        Some((source_id, content))
                    })
                    .as_ref();

                let (source, label_offset, label_len) = match resolved {
                    Some((source_id, content)) => {
                        let line = if frame.line > 0 { frame.line } else { 0 };
                        let pos = LineColumnPosition {
                            line: line as u32,
                            column: 0,
                            source: *source_id,
                            encoding: Utf8,
                        };
                        let line_start =
                            pos.into_scalar_position(&fs).0 as usize;
                        let line_text = &content[line_start..];
                        let line_end =
                            line_text.find('\n').unwrap_or(line_text.len());
                        let line_text = &line_text[..line_end];
                        let (byte_offset, line_len) = if let Some(name_pos) =
                            line_text.find(&frame.msg)
                        {
                            (line_start + name_pos, frame.msg.len())
                        } else {
                            let leading =
                                line_text.len() - line_text.trim_start().len();
                            (line_start + leading, line_text.trim().len())
                        };
                        let path = self
                            .intern_table
                            .get(frame.file as usize)
                            .map(|s| s.as_str())
                            .unwrap_or("<unknown>");
                        (
                            Some(miette::NamedSource::new(
                                path,
                                content.clone(),
                            )),
                            byte_offset,
                            line_len,
                        )
                    }
                    None => (None, 0, 0),
                };

                TraceDiagnostic {
                    msg: frame.msg.clone(),
                    level: frame.level,
                    source,
                    label_offset,
                    label_len,
                    fields: self.resolve_fields(frame),
                }
            })
            .collect()
    }

    /// Returns resolved diagnostics for each frame (miette without source-info).
    #[cfg(all(feature = "miette", not(feature = "source-info")))]
    pub(crate) fn diagnostics(&self) -> Vec<TraceDiagnostic> {
        self.frames
            .iter()
            .map(|frame| TraceDiagnostic {
                msg: frame.msg.clone(),
                level: frame.level,
                source: None,
                label_offset: 0,
                label_len: 0,
                fields: self.resolve_fields(frame),
            })
            .collect()
    }

    /// Returns resolved diagnostics for each frame (plain-text, no miette).
    #[cfg(not(feature = "miette"))]
    pub(crate) fn diagnostics(&self) -> Vec<TraceDiagnostic> {
        self.frames
            .iter()
            .map(|frame| {
                let file = self
                    .intern_table
                    .get(frame.file as usize)
                    .filter(|s| !s.is_empty())
                    .cloned();
                TraceDiagnostic {
                    msg: frame.msg.clone(),
                    level: frame.level,
                    fields: self.resolve_fields(frame),
                    file,
                    line: frame.line,
                }
            })
            .collect()
    }

    /// Returns the severity of the backtrace, which is the maximum level of all frames.
    #[cfg(feature = "miette")]
    pub(crate) fn severity(&self) -> Option<Severity> {
        match self
            .frames
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or(Level::ERROR)
        {
            Level::DEBUG | Level::TRACE | Level::INFO => None,
            Level::WARN => Some(Severity::Warning),
            Level::ERROR => Some(Severity::Error),
        }
    }
}

type Interner = StringInterner<BucketBackend<SymbolU16>>;

// r[impl jetstream.error.v2.backtrace.from-spantrace]
// r[impl jetstream.error.v2.span-trace.format]
pub(crate) fn backtrace_from_spantrace(span: &SpanTrace) -> Box<Backtrace> {
    match span.status() {
        tracing_error::SpanTraceStatus::CAPTURED => {
            let mut interned_strings = Interner::new();
            // Reserve index 0 for "" so that the 0 sentinel used for
            // missing file/module never collides with a real string.
            interned_strings.get_or_intern("");
            let mut spans = Vec::new();
            span.with_spans(|m, s| {
                let file = if let Some(file) = m.file() {
                    interned_strings.get_or_intern(file).to_usize() as u16
                } else {
                    0
                };
                let module = if let Some(module) = m.module_path() {
                    interned_strings.get_or_intern(module).to_usize() as u16
                } else {
                    0
                };
                let target = interned_strings
                    .get_or_intern(m.target())
                    .to_usize() as u16;
                let name =
                    interned_strings.get_or_intern(m.name()).to_usize() as u16;
                // Parse the formatted fields string into key=value pairs
                // The format from tracing is "key=value key2=value2"
                let fields: Vec<FieldPair> = if s.is_empty() {
                    Vec::new()
                } else {
                    s.split_whitespace()
                        .filter_map(|kv| {
                            let (k, v) = kv.split_once('=')?;
                            Some(FieldPair {
                                key: interned_strings
                                    .get_or_intern(k)
                                    .to_usize()
                                    as u16,
                                value: interned_strings
                                    .get_or_intern(v)
                                    .to_usize()
                                    as u16,
                            })
                        })
                        .collect()
                };

                let msg = m.name().to_string();
                spans.push(Frame {
                    msg,
                    name,
                    target,
                    module,
                    file,
                    line: m.line().unwrap_or(0) as u16,
                    fields,
                    level: *m.level(),
                });
                true
            });
            let mut intern_table = vec![String::new(); interned_strings.len()];
            for (sym, s) in interned_strings.iter() {
                intern_table[sym.to_usize()] = s.to_string();
            }

            Box::new(Backtrace {
                intern_table,
                frames: spans,
            })
        }
        _ => Box::new(Backtrace::default()),
    }
}
