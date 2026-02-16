use std::time::{SystemTime, UNIX_EPOCH};

use tracing::error;
use twilight_http::request::AuditLogReason as _;
use twilight_model::{
    gateway::payload::incoming::MessageCreate, guild::Permissions, util::Timestamp,
};

use crate::CommandMeta;
use crate::moderation::embeds::{fetch_target_profile, moderation_action_embed};
use rusty_core::Context;
use rusty_utils::parse::{parse_duration_seconds, parse_target_user_id};
use rusty_utils::permissions::has_message_permission;

pub const META: CommandMeta = CommandMeta {
    name: "timeout",
    desc: "Timeout a user for a duration (default: 10m).",
    category: "moderation",
    usage: "!timeout <user> [duration] [reason]",
};

const DEFAULT_TIMEOUT_SECS: u64 = 10 * 60;

/// Apply a temporary communication timeout to a target user.
pub async fn run(
    ctx: Context,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
    arg_tail: Option<&str>,
) -> anyhow::Result<()> {
    let http = &ctx.http;
    let Some(guild_id) = msg.guild_id else {
        http.create_message(msg.channel_id)
            .content("This command only works in servers.")
            .await?;
        return Ok(());
    };

    if !has_message_permission(http, &msg, Permissions::MODERATE_MEMBERS).await? {
        http.create_message(msg.channel_id)
            .content("You are not permitted to use this command.")
            .await?;
        return Ok(());
    }

    let Some(raw_target) = arg1 else {
        let usage = format!("Usage: `{}`", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    let Some(target_user_id) = parse_target_user_id(raw_target) else {
        let usage = format!("Usage: `{}`", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    if target_user_id == msg.author.id {
        http.create_message(msg.channel_id)
            .content("You can't timeout yourself.")
            .await?;
        return Ok(());
    }

    let (duration_secs, duration_label, reason) = match arg_tail {
        Some(tail) => {
            let mut parts = tail.splitn(2, char::is_whitespace);
            let first = parts.next().unwrap_or("");
            if let Some(parsed_duration) = parse_duration_seconds(first) {
                let parsed_reason = parts
                    .next()
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                (parsed_duration, first.to_owned(), parsed_reason)
            } else {
                (DEFAULT_TIMEOUT_SECS, "10m".to_owned(), Some(tail))
            }
        }
        None => (DEFAULT_TIMEOUT_SECS, "10m".to_owned(), None),
    };

    let expires_at_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
        .saturating_add(duration_secs);

    let Ok(expires_at) = Timestamp::from_secs(expires_at_secs as i64) else {
        http.create_message(msg.channel_id)
            .content("Unable to compute timeout expiration timestamp.")
            .await?;
        return Ok(());
    };

    let mut request = http
        .update_guild_member(guild_id, target_user_id)
        .communication_disabled_until(Some(expires_at));

    if let Some(reason) = reason {
        request = request.reason(reason);
    }

    if let Err(source) = request.await {
        error!(?source, "timeout request failed");
        http.create_message(msg.channel_id)
            .content("I couldn't timeout that user. Check role hierarchy and permissions.")
            .await?;
        return Ok(());
    }

    let target_profile = fetch_target_profile(http, target_user_id).await;
    let embed = moderation_action_embed(
        &target_profile,
        target_user_id,
        "timed out",
        reason,
        Some(&duration_label),
    )?;
    http.create_message(msg.channel_id).embeds(&[embed]).await?;

    Ok(())
}
