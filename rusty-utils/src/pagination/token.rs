//! Stateless pagination token encoding, parsing, and validation.

use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_PREFIX: &str = "pg";
const MODAL_TOKEN_PREFIX: &str = "pgm";

/// Parsed pagination token data from a button custom ID.
#[derive(Debug, Clone)]
pub struct PaginationToken {
    /// Logical command name (e.g. `permissions`).
    pub command: String,
    /// Button action (`prev` or `next`).
    pub action: String,
    /// Target page number, 1-based.
    pub page: usize,
    /// Total page count.
    pub total_pages: usize,
    /// User ID that owns this pagination session.
    pub user_id: u64,
    /// Expiry timestamp (unix seconds).
    pub expires_at: u64,
}

/// Parsed pagination jump-modal token data from a modal custom ID.
#[derive(Debug, Clone)]
pub struct PaginationModalToken {
    /// Logical command name (e.g. `permissions` or `help|utility`).
    pub command: String,
    /// Total page count at modal-open time.
    pub total_pages: usize,
    /// User ID that owns this pagination session.
    pub user_id: u64,
    /// Expiry timestamp (unix seconds).
    pub expires_at: u64,
}

/// Validation outcome for pagination button presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationValidationError {
    Invalid,
    WrongCommand,
    WrongUser,
    Expired,
    OutOfRange,
}

/// Build a compact custom ID carrying stateless pagination state.
pub fn build_custom_id(
    command: &str,
    action: &str,
    target_page: usize,
    total_pages: usize,
    user_id: u64,
    expires_at: u64,
) -> String {
    format!("{TOKEN_PREFIX}:{command}:{action}:{target_page}:{total_pages}:{user_id}:{expires_at}")
}

/// Parse a pagination custom ID.
pub fn parse_custom_id(custom_id: &str) -> Option<PaginationToken> {
    let mut parts = custom_id.split(':');

    let prefix = parts.next()?;
    if prefix != TOKEN_PREFIX {
        return None;
    }

    let command = parts.next()?.to_owned();
    let action = parts.next()?.to_owned();
    let page = parts.next()?.parse::<usize>().ok()?;
    let total_pages = parts.next()?.parse::<usize>().ok()?;
    let user_id = parts.next()?.parse::<u64>().ok()?;
    let expires_at = parts.next()?.parse::<u64>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some(PaginationToken {
        command,
        action,
        page,
        total_pages,
        user_id,
        expires_at,
    })
}

/// Validate a pagination token for command/user/expiry/page bounds.
pub fn validate_custom_id(
    custom_id: &str,
    expected_command: &str,
    actor_user_id: u64,
) -> Result<PaginationToken, PaginationValidationError> {
    let token = parse_custom_id(custom_id).ok_or(PaginationValidationError::Invalid)?;

    if token.command != expected_command {
        return Err(PaginationValidationError::WrongCommand);
    }

    if token.user_id != actor_user_id {
        return Err(PaginationValidationError::WrongUser);
    }

    if token.action != "prev" && token.action != "next" && token.action != "jump" {
        return Err(PaginationValidationError::Invalid);
    }

    if now_unix_secs() > token.expires_at {
        return Err(PaginationValidationError::Expired);
    }

    if token.page == 0 || token.page > token.total_pages {
        return Err(PaginationValidationError::OutOfRange);
    }

    Ok(token)
}

/// Build a modal custom ID carrying pagination session state.
pub fn build_modal_custom_id(
    command: &str,
    total_pages: usize,
    user_id: u64,
    expires_at: u64,
) -> String {
    format!("{MODAL_TOKEN_PREFIX}:{command}:{total_pages}:{user_id}:{expires_at}")
}

/// Parse a pagination modal custom ID.
pub fn parse_modal_custom_id(custom_id: &str) -> Option<PaginationModalToken> {
    let mut parts = custom_id.split(':');

    let prefix = parts.next()?;
    if prefix != MODAL_TOKEN_PREFIX {
        return None;
    }

    let command = parts.next()?.to_owned();
    let total_pages = parts.next()?.parse::<usize>().ok()?;
    let user_id = parts.next()?.parse::<u64>().ok()?;
    let expires_at = parts.next()?.parse::<u64>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some(PaginationModalToken {
        command,
        total_pages,
        user_id,
        expires_at,
    })
}

/// Whether the provided unix timestamp is already expired.
pub fn is_expired(expires_at: u64) -> bool {
    now_unix_secs() > expires_at
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
