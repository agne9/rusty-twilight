use tracing::error;
use twilight_model::{
    gateway::payload::incoming::{InteractionCreate, MessageCreate},
    guild::Permissions,
    id::{Id, marker::UserMarker},
};

use crate::CommandMeta;
use crate::moderation::embeds::{
    fetch_target_profile, guild_only_message, moderation_action_embed,
    moderation_invalid_interaction_message, moderation_permission_combo_denied_message,
    moderation_self_action_message, usage_message,
};
use rusty_core::Context;
use rusty_utils::cleanup::purge_user_globally;
use rusty_utils::interaction::{
    ConfirmationAction, build_confirmation_components, build_confirmation_custom_ids,
    edit_original_response_content_embed_without_components, parse_confirmation_custom_id,
    respond_ephemeral_notice, respond_update_content_embed_without_components,
    respond_update_without_components,
};
use rusty_utils::parse::{parse_duration_seconds, parse_target_user_id};
use rusty_utils::permissions::{check_interaction_permissions, has_message_permission};
use rusty_utils::time::now_unix_secs;

pub const META: CommandMeta = CommandMeta {
    name: "terminate",
    desc: "Ban a user and purge their messages (optionally by period and reason).",
    category: "moderation",
    usage: "!terminate <user> [period] [reason]",
};

const CUSTOM_ID_PREFIX: &str = "terminate:";

pub async fn run(
    ctx: Context,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
    arg_tail: Option<&str>,
) -> anyhow::Result<()> {
    let http = &ctx.http;
    if msg.guild_id.is_none() {
        http.create_message(msg.channel_id)
            .content(guild_only_message())
            .await?;
        return Ok(());
    }

    let required_permissions = Permissions::BAN_MEMBERS | Permissions::MANAGE_MESSAGES;
    if !has_message_permission(http, &msg, required_permissions).await? {
        let denied = moderation_permission_combo_denied_message("Ban Members and Manage Messages");
        http.create_message(msg.channel_id).content(&denied).await?;
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

    if target_user_id == msg.author.id {
        let self_message = moderation_self_action_message("terminate");
        http.create_message(msg.channel_id)
            .content(&self_message)
            .await?;
        return Ok(());
    }

    let (cutoff_secs, cutoff_display, reason) = match arg_tail {
        Some(tail) => {
            let mut parts = tail.splitn(2, char::is_whitespace);
            let first = parts.next().unwrap_or("");

            if let Some(duration_secs) = parse_duration_seconds(first) {
                let parsed_reason = parts
                    .next()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                (
                    Some(now_unix_secs().saturating_sub(duration_secs)),
                    first.to_owned(),
                    parsed_reason,
                )
            } else {
                (None, "all-time".to_owned(), Some(tail.to_owned()))
            }
        }
        None => (None, "all-time".to_owned(), None),
    };

    let (confirm_custom_id, decline_custom_id) = build_confirmation_custom_ids(
        CUSTOM_ID_PREFIX,
        msg.author.id.get(),
        target_user_id.get(),
        cutoff_secs,
    );

    let components = build_confirmation_components(confirm_custom_id, decline_custom_id);
    let target_profile = fetch_target_profile(http, target_user_id).await;
    let confirmation = moderation_action_embed(
        &target_profile,
        target_user_id,
        "queued for termination",
        reason.as_deref(),
        None,
    )?;
    let confirmation_text = format!(
        "Ban and purge pending moderator confirmation.\nPeriod: {}",
        cutoff_display
    );

    http.create_message(msg.channel_id)
        .content(&confirmation_text)
        .embeds(&[confirmation])
        .components(&components)
        .await?;

    Ok(())
}

pub async fn handle_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;

    let Some(component_data) = interaction.data.as_ref().and_then(|data| {
        if let twilight_model::application::interaction::InteractionData::MessageComponent(
            component,
        ) = data
        {
            Some(component)
        } else {
            None
        }
    }) else {
        return Ok(false);
    };

    if !component_data.custom_id.starts_with(CUSTOM_ID_PREFIX) {
        return Ok(false);
    }

    let Some(parsed) = parse_confirmation_custom_id(&component_data.custom_id, CUSTOM_ID_PREFIX)
    else {
        let invalid = moderation_invalid_interaction_message("terminate");
        respond_update_without_components(http, &interaction, &invalid).await?;
        return Ok(true);
    };

    let Some(actor_id) = interaction.author_id().map(|id| id.get()) else {
        respond_ephemeral_notice(http, &interaction, "Unable to determine interaction user.")
            .await?;
        return Ok(true);
    };

    if actor_id != parsed.requester_id {
        respond_ephemeral_notice(
            http,
            &interaction,
            "Only the user who initiated this terminate action can confirm it.",
        )
        .await?;
        return Ok(true);
    }

    let Some(guild_id) = interaction.guild_id else {
        respond_ephemeral_notice(http, &interaction, guild_only_message()).await?;
        return Ok(true);
    };

    let required_permissions = Permissions::BAN_MEMBERS | Permissions::MANAGE_MESSAGES;
    if !check_interaction_permissions(&interaction, required_permissions) {
        respond_ephemeral_notice(
            http,
            &interaction,
            "You no longer have the required permissions for this action.",
        )
        .await?;
        return Ok(true);
    }

    let target_user_id = Id::<UserMarker>::new(parsed.target_id);
    let target_profile = fetch_target_profile(http, target_user_id).await;

    match parsed.action {
        ConfirmationAction::Decline => {
            let cancelled_embed = moderation_action_embed(
                &target_profile,
                target_user_id,
                "left unchanged",
                Some("Termination cancelled."),
                None,
            )?;
            respond_update_content_embed_without_components(
                http,
                &interaction,
                "Termination cancelled.",
                &cancelled_embed,
            )
            .await?;
            return Ok(true);
        }
        ConfirmationAction::Confirm => {
            let loading_embed = moderation_action_embed(
                &target_profile,
                target_user_id,
                "queued for termination",
                Some("Termination in progress."),
                None,
            )?;
            respond_update_content_embed_without_components(
                http,
                &interaction,
                "Terminating...",
                &loading_embed,
            )
            .await?;
        }
    }

    if let Err(source) = http.create_ban(guild_id, target_user_id).await {
        error!(?source, "terminate ban failed");
        let failed_embed = moderation_action_embed(
            &target_profile,
            target_user_id,
            "not terminated",
            Some("Ban failed. Check hierarchy and permissions."),
            None,
        )?;
        edit_original_response_content_embed_without_components(
            http,
            &interaction,
            "Ban failed. Check hierarchy and permissions.",
            &failed_embed,
        )
        .await?;
        return Ok(true);
    }

    let deleted_count = purge_user_globally(http, guild_id, target_user_id, parsed.context_value)
        .await
        .unwrap_or_else(|source| {
            error!(?source, "terminate purge failed");
            0
        });

    let window = parsed
        .context_value
        .map(|cutoff| format!("since <t:{}:R>", cutoff))
        .unwrap_or_else(|| "all accessible history".to_owned());
    let applied_text = format!(
        "Ban applied; deleted {} message(s).\nPeriod: {}",
        deleted_count, window
    );
    let success_embed = moderation_action_embed(
        &target_profile,
        target_user_id,
        "terminated",
        Some("Ban applied and purge completed."),
        None,
    )?;
    edit_original_response_content_embed_without_components(
        http,
        &interaction,
        &applied_text,
        &success_embed,
    )
    .await?;

    Ok(true)
}
