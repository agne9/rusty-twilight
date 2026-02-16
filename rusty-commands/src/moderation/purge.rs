use tokio::time::{Duration, sleep};
use tracing::error;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    guild::Permissions,
    id::{Id, marker::MessageMarker},
};

use crate::CommandMeta;
use rusty_core::Context;
use rusty_utils::permissions::has_message_permission;

pub const META: CommandMeta = CommandMeta {
    name: "purge",
    desc: "Delete the latest messages in this channel.",
    category: "moderation",
    usage: "!purge <amount>",
};

const MAX_PURGE: u16 = 100;

/// Delete a bounded number of recent channel messages.
pub async fn run(ctx: Context, msg: Box<MessageCreate>, arg1: Option<&str>) -> anyhow::Result<()> {
    let http = &ctx.http;
    let Some(requested_raw) = arg1 else {
        let usage = format!("Usage: `{}`", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    let Ok(requested) = requested_raw.parse::<u16>() else {
        http.create_message(msg.channel_id)
            .content("Amount must be a number between 1 and 100.")
            .await?;
        return Ok(());
    };

    if requested == 0 {
        http.create_message(msg.channel_id)
            .content("Amount must be at least 1.")
            .await?;
        return Ok(());
    };

    let amount = requested.min(MAX_PURGE);
    let delete_count = amount.saturating_add(1).min(MAX_PURGE);

    if !has_message_permission(http, &msg, Permissions::MANAGE_MESSAGES).await? {
        http.create_message(msg.channel_id)
            .content("You are not permitted to use this command.")
            .await?;
        return Ok(());
    }

    let messages = http
        .channel_messages(msg.channel_id)
        .limit(delete_count)
        .await?
        .model()
        .await?;

    let ids: Vec<Id<MessageMarker>> = messages.into_iter().map(|m| m.id).collect();

    if ids.is_empty() {
        http.create_message(msg.channel_id)
            .content("No messages found to delete.")
            .await?;
        return Ok(());
    }

    let delete_result = if ids.len() == 1 {
        http.delete_message(msg.channel_id, ids[0]).await
    } else {
        http.delete_messages(msg.channel_id, &ids).await
    };

    if let Err(source) = delete_result {
        error!(?source, "purge delete request failed");
        http.create_message(msg.channel_id)
            .content("I couldn't delete messages. I likely need the 'Manage Messages' permission.")
            .await?;
        return Ok(());
    }

    let confirmation = format!("Purged {} message(s).", amount);
    let confirmation_message = http
        .create_message(msg.channel_id)
        .content(&confirmation)
        .await?
        .model()
        .await?;

    sleep(Duration::from_secs(3)).await;
    let _ = http
        .delete_message(msg.channel_id, confirmation_message.id)
        .await;

    Ok(())
}
