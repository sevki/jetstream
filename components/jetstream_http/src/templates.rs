use askama::Template;

#[derive(Template)]
#[template(path = "template.html")]
pub struct JetStreamTemplate<'a> {
    pub body: &'a str,
    pub version: &'static str,
    pub year: u16,
}

impl Default for JetStreamTemplate<'_> {
    fn default() -> Self {
        Self {
            body: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
            year: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| 1970 + (d.as_secs() / 31_536_000) as u16)
                .unwrap_or(2026),
        }
    }
}
