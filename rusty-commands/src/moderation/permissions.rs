use std::sync::Arc;
use twilight_model::gateway::payload::incoming::{InteractionCreate, MessageCreate};

use crate::CommandMeta;
use rusty_core::Context;
use rusty_utils::pagination::{
    DEFAULT_TIMEOUT_SECS, PaginationInteractionValidation, PaginationModalSubmitValidation,
    build_paginated_list_view, clamp_page, open_jump_modal_from_token, parse_one_based_page,
    resolve_modal_target_page, respond_ephemeral_message, send_paginated_message, total_pages,
    update_paginated_interaction_message, validate_interaction_for_command,
    validate_jump_modal_for_command,
};
use rusty_utils::permissions::{permission_names, resolve_message_author_permissions};

pub const META: CommandMeta = CommandMeta {
    name: "permissions",
    desc: "Display your server permissions.",
    category: "moderation",
    usage: "!permissions [page]",
};

const PERMISSIONS_PER_PAGE: usize = 10;

/// Display invoking member permissions in a paginated embed.
pub async fn run(ctx: Context, msg: Box<MessageCreate>, arg1: Option<&str>) -> anyhow::Result<()> {
    let http = &ctx.http;
    let perms = match resolve_message_author_permissions(http, &msg).await? {
        Some(perms) => perms,
        None => {
            http.create_message(msg.channel_id)
                .content("This command only works in servers.")
                .await?;
            return Ok(());
        }
    };

    if perms.is_empty() {
        http.create_message(msg.channel_id)
            .content("No permissions found for your member record.")
            .await?;
        return Ok(());
    }

    let names = permission_names(perms);

    if names.is_empty() {
        http.create_message(msg.channel_id)
            .content("You have no permissions set.")
            .await?;
        return Ok(());
    }

    let total_pages = total_pages(names.len(), PERMISSIONS_PER_PAGE);
    let Some(requested_page) = parse_one_based_page(arg1) else {
        let usage = format!("Usage: `{}` (page starts at 1)", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    if requested_page > total_pages {
        let msg_out = format!(
            "Page {} does not exist. Available pages: 1-{}.",
            requested_page, total_pages
        );
        http.create_message(msg.channel_id)
            .content(&msg_out)
            .await?;
        return Ok(());
    }

    let (embed, components) = build_paginated_list_view(
        "permissions",
        "Your Server Permissions",
        &names,
        requested_page,
        PERMISSIONS_PER_PAGE,
        msg.author.id.get(),
        DEFAULT_TIMEOUT_SECS,
    )?;

    send_paginated_message(
        Arc::clone(&ctx.http),
        msg.channel_id,
        embed,
        components,
        total_pages,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(())
}

/// Handle pagination button presses for the `permissions` command.
pub async fn handle_pagination_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, token) =
        match validate_interaction_for_command(http, &interaction, "permissions").await? {
            PaginationInteractionValidation::NotForCommand => return Ok(false),
            PaginationInteractionValidation::HandledInvalid => return Ok(true),
            PaginationInteractionValidation::Valid {
                actor_user_id,
                token,
            } => (actor_user_id, token),
        };

    let Some(perms) = interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
    else {
        respond_ephemeral_message(
            http,
            &interaction,
            "Unable to resolve member permissions for this interaction.",
        )
        .await?;
        return Ok(true);
    };

    let names = permission_names(perms);
    if names.is_empty() {
        respond_ephemeral_message(http, &interaction, "No permissions available for display.")
            .await?;
        return Ok(true);
    }

    let total_pages = total_pages(names.len(), PERMISSIONS_PER_PAGE);

    if token.action == "jump" {
        open_jump_modal_from_token(http, &interaction, &token, total_pages).await?;
        return Ok(true);
    }

    let target_page = clamp_page(token.page, total_pages);
    let (embed, components) = build_paginated_list_view(
        "permissions",
        "Your Server Permissions",
        &names,
        target_page,
        PERMISSIONS_PER_PAGE,
        actor_id,
        DEFAULT_TIMEOUT_SECS,
    )?;

    update_paginated_interaction_message(
        Arc::clone(&ctx.http),
        &interaction,
        embed,
        components,
        total_pages,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(true)
}

/// Handle jump-modal submit interactions for the `permissions` command.
pub async fn handle_pagination_modal_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, entered_page, total_pages_hint) =
        match validate_jump_modal_for_command(http, &interaction, "permissions").await? {
            PaginationModalSubmitValidation::NotForCommand => return Ok(false),
            PaginationModalSubmitValidation::HandledInvalid => return Ok(true),
            PaginationModalSubmitValidation::Valid {
                actor_user_id,
                requested_page,
                total_pages_hint,
                ..
            } => (actor_user_id, requested_page, total_pages_hint),
        };

    let Some(perms) = interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
    else {
        respond_ephemeral_message(
            http,
            &interaction,
            "Unable to resolve member permissions for this interaction.",
        )
        .await?;
        return Ok(true);
    };

    let names = permission_names(perms);
    if names.is_empty() {
        respond_ephemeral_message(http, &interaction, "No permissions available for display.")
            .await?;
        return Ok(true);
    }

    let total_pages: usize = total_pages(names.len(), PERMISSIONS_PER_PAGE);
    let target_page = resolve_modal_target_page(entered_page, total_pages, total_pages_hint);

    let (embed, components) = build_paginated_list_view(
        "permissions",
        "Your Server Permissions",
        &names,
        target_page,
        PERMISSIONS_PER_PAGE,
        actor_id,
        DEFAULT_TIMEOUT_SECS,
    )?;

    update_paginated_interaction_message(
        Arc::clone(&ctx.http),
        &interaction,
        embed,
        components,
        total_pages,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(true)
}
