use std::sync::Arc;

use rusty_database::Database;
use twilight_http::Client;

/// Shared application context passed into command handlers.
///
/// Cheap to clone because it only stores reference-counted shared state.
#[derive(Clone)]
pub struct Context {
    pub db: Database,
    pub http: Arc<Client>,
}

impl Context {
    /// Create a new application context.
    pub fn new(http: Arc<Client>, db: Database) -> Self {
        Self { http, db }
    }
}
