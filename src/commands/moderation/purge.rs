use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    guild::Permissions,
    id::{Id, marker::MessageMarker},
};

use crate::commands::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "purge",
    desc: "Delete the latest messages in this channel.",
    category: "moderation",
};

const MAX_PURGE: u16 = 100;

pub async fn run(
    http: Arc<Client>,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
) -> anyhow::Result<()> {
    let Some(requested_raw) = arg1 else {
        http.create_message(msg.channel_id)
            .content("Usage: !purge <amount>")
            .await?;
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

    if let Some(p) = msg.member.as_ref().and_then(|m| m.permissions) {
        let user_has_manage_messages =
            p.contains(Permissions::MANAGE_MESSAGES) || p.contains(Permissions::ADMINISTRATOR);

        if !user_has_manage_messages {
            http.create_message(msg.channel_id)
                .content("You are not permitted to use this command.")
                .await?;
            return Ok(());
        }
    }

    let messages = http
        .channel_messages(msg.channel_id)
        .limit(amount)
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

    if delete_result.is_err() {
        http.create_message(msg.channel_id)
            .content("I couldn't delete messages. I likely need the 'Manage Messages' permission.")
            .await?;
        return Ok(());
    }

    let confirmation = format!("Purged {} message(s).", amount);
    http.create_message(msg.channel_id)
        .content(&confirmation)
        .await?;

    Ok(())
}
