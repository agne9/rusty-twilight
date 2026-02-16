use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

use crate::CommandMeta;
use crate::moderation::embeds::{fetch_target_profile, moderation_action_embed};
use rusty_core::Context;
use rusty_database::warnings::record_warning;
use rusty_utils::parse::parse_target_user_id;
use rusty_utils::permissions::has_message_permission;

pub const META: CommandMeta = CommandMeta {
    name: "warn",
    desc: "Issue a warning to a user.",
    category: "moderation",
    usage: "!warn <user> [reason]",
};

/// Record a warning for a target user and report it back to the channel.
pub async fn run(
    ctx: Context,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
    arg_tail: Option<&str>,
) -> anyhow::Result<()> {
    let http = &ctx.http;
    let Some(_guild_id) = msg.guild_id else {
        http.create_message(msg.channel_id)
            .content("This command only works in servers.")
            .await?;
        return Ok(());
    };

    if !has_message_permission(http, &msg, Permissions::MANAGE_MESSAGES).await? {
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
    let warning =
        record_warning(&ctx.db, target_user_id.get(), msg.author.id.get(), reason).await?;
    let action = format!("warned #{}", warning.warn_number);

    let target_profile = fetch_target_profile(http, target_user_id).await;
    let embed =
        moderation_action_embed(&target_profile, target_user_id, &action, Some(reason), None)?;
    http.create_message(msg.channel_id).embeds(&[embed]).await?;

    Ok(())
}
