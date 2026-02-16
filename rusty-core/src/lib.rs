use std::sync::Arc;

use twilight_http::Client;

/// Shared application context passed into command handlers.
///
/// Cheap to clone because it only stores reference-counted shared state.
#[derive(Clone)]
pub struct Context {
    pub http: Arc<Client>,
}

impl Context {
    /// Create a new application context.
    pub fn new(http: Arc<Client>) -> Self {
        Self { http }
    }
}
