use sqlx::{migrate::Migrator, PgPool};

/// Compile-time discovered SQLx migrations for the `rusty-database` crate.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// Shared database handle passed across crates.
#[derive(Clone)]
pub struct Database {
	pool: PgPool,
}

impl Database {
	/// Create a database handle from an existing pool.
	pub fn new(pool: PgPool) -> Self {
		Self { pool }
	}

	/// Expose the underlying pool for query modules.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}
}

/// Database-backed state stores shared by command handlers.
pub mod warnings;
