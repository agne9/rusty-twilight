use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context as _;

use crate::Database;

#[derive(Clone, Debug)]
pub struct WarningEntry {
    pub warned_at: u64,
    pub moderator_id: u64,
    pub reason: String,
}

#[derive(sqlx::FromRow)]
struct WarningRow {
    warned_at: i64,
    moderator_id: i64,
    reason: String,
}

#[derive(Clone, Copy, Debug)]
pub struct WarningRecord {
    pub warn_number: usize,
}

/// Record a warning for a target user and return the new warning number.
pub async fn record_warning(
    db: &Database,
    user_id: u64,
    moderator_id: u64,
    reason: &str,
) -> anyhow::Result<WarningRecord> {
    let warned_at = now_unix_secs();
    let user_id_i64 = i64::try_from(user_id).context("user_id out of i64 range")?;
    let moderator_id_i64 = i64::try_from(moderator_id).context("moderator_id out of i64 range")?;
    let warned_at_i64 = i64::try_from(warned_at).context("warned_at out of i64 range")?;

    sqlx::query(
        "INSERT INTO warnings (user_id, moderator_id, reason, warned_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id_i64)
    .bind(moderator_id_i64)
    .bind(reason)
    .bind(warned_at_i64)
    .execute(db.pool())
    .await?;

    let warn_number: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM warnings WHERE user_id = $1")
        .bind(user_id_i64)
        .fetch_one(db.pool())
        .await?;

    let warn_number = usize::try_from(warn_number).context("warn count out of usize range")?;

    Ok(WarningRecord { warn_number })
}

/// Return warning entries for a target user in the inclusive [since, now] range.
pub async fn warnings_since(
    db: &Database,
    user_id: u64,
    since: u64,
) -> anyhow::Result<Vec<WarningEntry>> {
    let user_id_i64 = i64::try_from(user_id).context("user_id out of i64 range")?;
    let since_i64 = i64::try_from(since).context("since out of i64 range")?;

    let rows: Vec<WarningRow> = sqlx::query_as(
        "SELECT warned_at, moderator_id, reason
         FROM warnings
         WHERE user_id = $1 AND warned_at >= $2
         ORDER BY warned_at ASC",
    )
    .bind(user_id_i64)
    .bind(since_i64)
    .fetch_all(db.pool())
    .await?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let warned_at = u64::try_from(row.warned_at).context("warned_at row out of u64 range")?;
        let moderator_id =
            u64::try_from(row.moderator_id).context("moderator_id row out of u64 range")?;
        entries.push(WarningEntry {
            warned_at,
            moderator_id,
            reason: row.reason,
        });
    }

    entries.sort_by_key(|entry| entry.warned_at);
    Ok(entries)
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
