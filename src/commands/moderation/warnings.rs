use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    guild::Permissions,
    id::{Id, marker::UserMarker},
};
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, ImageSource};

use crate::commands::CommandMeta;
use crate::embed::embed::DEFAULT_EMBED_COLOR;
use crate::services::parse::parse_target_user_id;
use crate::services::permissions::has_message_permission;
use crate::services::warnings::{now_unix_secs, warnings_since};

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

pub async fn run(
    http: Arc<Client>,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
    arg_tail: Option<&str>,
) -> anyhow::Result<()> {
    let Some(_guild_id) = msg.guild_id else {
        http.create_message(msg.channel_id)
            .content("This command only works in servers.")
            .await?;
        return Ok(());
    };

    if !has_message_permission(&http, &msg, Permissions::MANAGE_MESSAGES).await? {
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

    let window = parse_window(arg_tail);
    let (since, window_label) = match window {
        WarningWindow::Days(days) => (
            now_unix_secs().saturating_sub(days.saturating_mul(86_400)),
            format!("last {} day(s)", days),
        ),
        WarningWindow::All => (0, "all time".to_owned()),
    };

    let entries = warnings_since(target_user_id, since).await;
    let count = entries.len();
    let (display_name, avatar_url) = fetch_target_profile(&http, target_user_id).await;

    let mut description = format!("Total warnings in {}: **{}**\n\n", window_label, count);

    if entries.is_empty() {
        description.push_str("No warnings in this period.");
    } else {
        let start = entries.len().saturating_sub(5);
        for (index, entry) in entries.iter().enumerate().skip(start) {
            let line = format!(
                "#{idx} • <t:{ts}:F> • by <@{mod_id}>\nReason: {reason}\n\n",
                idx = index + 1,
                ts = entry.warned_at,
                mod_id = entry.moderator_id,
                reason = sanitize_reason(&entry.reason)
            );
            description.push_str(&line);
        }
    }

    let title = format!("Warnings for {}", display_name);
    let builder = EmbedBuilder::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    let builder = match avatar_url {
        Some(url) => {
            let icon = ImageSource::url(url)?;
            let author = EmbedAuthorBuilder::new(title).icon_url(icon).build();
            builder.author(author)
        }
        None => builder.title(title),
    };

    let embed = builder.validate()?.build();

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

fn sanitize_reason(reason: &str) -> String {
    reason.replace('@', "@\u{200B}")
}

async fn fetch_target_profile(http: &Client, user_id: Id<UserMarker>) -> (String, Option<String>) {
    let user = match http.user(user_id).await {
        Ok(response) => match response.model().await {
            Ok(model) => model,
            Err(_) => return (format!("User {}", user_id.get()), None),
        },
        Err(_) => return (format!("User {}", user_id.get()), None),
    };

    let display_name = user.global_name.unwrap_or(user.name);
    let avatar_url = Some(match user.avatar {
        Some(avatar) => format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=128",
            user_id.get(),
            avatar
        ),
        None => {
            let default_avatar_index = (user_id.get() >> 22) % 6;
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                default_avatar_index
            )
        }
    });

    (display_name, avatar_url)
}
