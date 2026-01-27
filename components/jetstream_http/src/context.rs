use axum::extract::FromRequestParts;
use http::request::Parts;
use jetstream_rpc::context::Context;
use std::ops::{Deref, DerefMut};

/// A wrapper around [`Context`] that implements Axum's [`FromRequestParts`].
///
/// This allows extracting the Jetstream context in Axum handlers:
///
/// ```ignore
/// async fn handler(ctx: JetStreamContext) {
///     println!("Remote: {:?}", ctx.remote());
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct JetStreamContext(pub Context);

impl Deref for JetStreamContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JetStreamContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Context> for JetStreamContext {
    fn from(ctx: Context) -> Self {
        Self(ctx)
    }
}

impl From<JetStreamContext> for Context {
    fn from(ctx: JetStreamContext) -> Self {
        ctx.0
    }
}

impl<S> FromRequestParts<S> for JetStreamContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .remove::<Context>()
            .map(JetStreamContext)
            .unwrap_or_default())
    }
}
