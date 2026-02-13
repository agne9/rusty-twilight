use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::{InteractionCreate, MessageCreate};

use crate::commands::CommandMeta;
use crate::services::permissions::{permission_names, resolve_message_author_permissions};
use crate::ui::pagination::{
    DEFAULT_TIMEOUT_SECS, PaginationInteractionValidation, build_paginated_list_view, clamp_page,
    parse_one_based_page, respond_ephemeral_message, respond_update_message, total_pages,
    validate_interaction_for_command,
};

pub const META: CommandMeta = CommandMeta {
    name: "permissions",
    desc: "Display your server permissions (paginated).",
    category: "moderation",
};

const PERMISSIONS_PER_PAGE: usize = 10;

pub async fn run(
    http: Arc<Client>,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
) -> anyhow::Result<()> {
    let perms = match resolve_message_author_permissions(&http, &msg).await? {
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
        http.create_message(msg.channel_id)
            .content("Usage: !permissions [page], where page starts at 1.")
            .await?;
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

    http.create_message(msg.channel_id)
        .embeds(&[embed])
        .components(&components)
        .await?;

    Ok(())
}

/// Handle pagination button presses for the `permissions` command.
pub async fn handle_pagination_interaction(
    http: Arc<Client>,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let (actor_id, token) =
        match validate_interaction_for_command(&http, &interaction, "permissions").await? {
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
            &http,
            &interaction,
            "Unable to resolve member permissions for this interaction.",
        )
        .await?;
        return Ok(true);
    };

    let names = permission_names(perms);
    if names.is_empty() {
        respond_ephemeral_message(&http, &interaction, "No permissions available for display.")
            .await?;
        return Ok(true);
    }

    let total_pages = total_pages(names.len(), PERMISSIONS_PER_PAGE);
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

    respond_update_message(&http, &interaction, &[embed], &components).await?;

    Ok(true)
}
