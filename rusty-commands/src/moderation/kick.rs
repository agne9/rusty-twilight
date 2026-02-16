use tracing::error;
use twilight_http::request::AuditLogReason as _;
use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

use crate::CommandMeta;
use crate::moderation::embeds::{fetch_target_profile, moderation_action_embed};
use rusty_core::Context;
use rusty_utils::parse::parse_target_user_id;
use rusty_utils::permissions::has_message_permission;

pub const META: CommandMeta = CommandMeta {
    name: "kick",
    desc: "Kick a user from the server.",
    category: "moderation",
    usage: "!kick <user> [reason]",
};

/// Kick a target user after permission and input validation.
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

    if !has_message_permission(http, &msg, Permissions::KICK_MEMBERS).await? {
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
            .content("You can't kick yourself.")
            .await?;
        return Ok(());
    }

    let mut request = http.remove_guild_member(guild_id, target_user_id);
    if let Some(reason) = arg_tail {
        request = request.reason(reason);
    }

    if let Err(source) = request.await {
        error!(?source, "kick request failed");
        http.create_message(msg.channel_id)
            .content("I couldn't kick that user. Check role hierarchy and permissions.")
            .await?;
        return Ok(());
    }

    let target_profile = fetch_target_profile(http, target_user_id).await;
    let embed = moderation_action_embed(&target_profile, target_user_id, "kicked", arg_tail, None)?;
    http.create_message(msg.channel_id).embeds(&[embed]).await?;

    Ok(())
}
