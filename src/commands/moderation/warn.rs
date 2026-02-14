use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

use crate::commands::CommandMeta;
use crate::services::moderation::send_moderation_action_embed;
use crate::services::parse::parse_target_user_id;
use crate::services::permissions::has_message_permission;
use crate::services::warnings::record_warning;

pub const META: CommandMeta = CommandMeta {
    name: "warn",
    desc: "Issue a warning to a user.",
    category: "moderation",
    usage: "!warn <user> [reason]",
};

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

    let reason = arg_tail.unwrap_or("No reason provided");
    let warning = record_warning(target_user_id, msg.author.id, reason).await;
    let action = format!("warned #{}", warning.warn_number);

    send_moderation_action_embed(
        &http,
        msg.channel_id,
        target_user_id,
        &action,
        Some(reason),
        None,
    )
    .await?;

    Ok(())
}
