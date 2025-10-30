use askama::Template;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    error_type: String,
    message: String,
    details: String,
}

#[derive(thiserror::Error, Debug)]
pub enum JetstreamProtocolError {
    #[error("Missing protocol header")]
    MissingProtocolHeader,
    #[error("Missing upgrade header")]
    MissingUpgradeHeader,
    #[error("Invalid upgrade header")]
    InvalidUpgradeHeader,
    #[error("Invalid protocol")]
    InvalidProtocol(String),
    #[error("Missing IP address")]
    MissingIPAddress,
}

impl JetstreamProtocolError {
    /// Returns the HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            JetstreamProtocolError::MissingProtocolHeader => 400,
            JetstreamProtocolError::MissingUpgradeHeader => 400,
            JetstreamProtocolError::InvalidUpgradeHeader => 400,
            JetstreamProtocolError::InvalidProtocol(_) => 400,
            JetstreamProtocolError::MissingIPAddress => 400,
        }
    }

    /// Returns a user-friendly error type string
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::MissingProtocolHeader => "Missing Protocol Header",
            Self::MissingUpgradeHeader => "Missing Upgrade Header",
            Self::InvalidUpgradeHeader => "Invalid Upgrade Header",
            Self::InvalidProtocol(_) => "Invalid Protocol",
            Self::MissingIPAddress => "Missing IP Address",
        }
    }

    /// Returns a user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            Self::MissingProtocolHeader => {
                "The request is missing the required X-JetStream-Proto header. Please ensure your client includes this header.".into()
            }
            Self::MissingUpgradeHeader => {
                "The request is missing the required Upgrade header for WebSocket connection.".into()
            }
            Self::InvalidUpgradeHeader => {
                "The Upgrade header must be set to 'websocket' for JetStream connections.".into()
            }
            Self::InvalidProtocol(proto) =>
                format!("Invalid protocol: {}", proto),
            Self::MissingIPAddress => {
                "The request is missing the required X-Forwarded-For header. Please ensure your client includes this header.".into()
            }
        }
    }
}

impl From<&JetstreamProtocolError> for ErrorTemplate {
    fn from(error: &JetstreamProtocolError) -> Self {
        Self {
            error_type: error.error_type().to_string(),
            message: error.user_message(),
            details: String::new(),
        }
    }
}

impl From<JetstreamProtocolError>
    for std::result::Result<worker::Response, worker::Error>
{
    fn from(val: JetstreamProtocolError) -> Self {
        let status_code = val.status_code();
        let template = ErrorTemplate::from(&val);

        // Render the error template as the body content
        let error_html = template
            .render()
            .map_err(|error| worker::Error::RustError(error.to_string()))?;

        // Wrap it in the main JetStream template
        let full_template = super::JetStreamTemplate {
            body: &error_html,
            version: env!("CARGO_PKG_VERSION"),
        };

        let html = full_template
            .render()
            .map_err(|error| worker::Error::RustError(error.to_string()))?;

        let response = worker::Response::from_html(html)
            .map_err(|error| worker::Error::RustError(error.to_string()))?;
        Ok(response.with_status(status_code))
    }
}
