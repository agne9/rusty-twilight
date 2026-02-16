use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

use crate::CommandMeta;
use crate::moderation::embeds::{
    fetch_target_profile, guild_only_message, permission_denied_message, usage_message,
    warnings_overview_embed, warnings_window_label_days,
};
use rusty_core::Context;
use rusty_database::warnings::{now_unix_secs, warnings_since};
use rusty_utils::parse::parse_target_user_id;
use rusty_utils::permissions::has_message_permission;

pub const META: CommandMeta = CommandMeta {
    name: "warnings",
    desc: "Show warning history for a user in a time window.",
    category: "moderation",
    usage: "!warnings <user> [days|all]",
};

const DEFAULT_DAYS: u64 = 30;

enum WarningWindow {
    Days(u64),
    All,
}

/// Show warning history for a target user within a selected time window.
pub async fn run(
    ctx: Context,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
    arg_tail: Option<&str>,
) -> anyhow::Result<()> {
    let http = &ctx.http;
    let Some(_guild_id) = msg.guild_id else {
        http.create_message(msg.channel_id)
            .content(guild_only_message())
            .await?;
        return Ok(());
    };

    if !has_message_permission(http, &msg, Permissions::MANAGE_MESSAGES).await? {
        http.create_message(msg.channel_id)
            .content(permission_denied_message())
            .await?;
        return Ok(());
    }

    let Some(raw_target) = arg1 else {
        let usage = usage_message(META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    let Some(target_user_id) = parse_target_user_id(raw_target) else {
        let usage = usage_message(META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    let window = parse_window(arg_tail);
    let (since, window_label) = match window {
        WarningWindow::Days(days) => (
            now_unix_secs().saturating_sub(days.saturating_mul(86_400)),
            warnings_window_label_days(days),
        ),
        WarningWindow::All => (0, "all time".to_owned()),
    };

    let entries = warnings_since(target_user_id.get(), since).await;
    let target_profile = fetch_target_profile(http, target_user_id).await;
    let embed = warnings_overview_embed(&target_profile, &window_label, &entries)?;

    http.create_message(msg.channel_id).embeds(&[embed]).await?;

    Ok(())
}

fn parse_window(arg_tail: Option<&str>) -> WarningWindow {
    let Some(raw) = arg_tail.and_then(|value| value.split_whitespace().next()) else {
        return WarningWindow::Days(DEFAULT_DAYS);
    };

    if raw.eq_ignore_ascii_case("all") {
        return WarningWindow::All;
    }

    let Some(days) = raw.parse::<u64>().ok().filter(|value| *value > 0) else {
        return WarningWindow::Days(DEFAULT_DAYS);
    };

    WarningWindow::Days(days)
}
