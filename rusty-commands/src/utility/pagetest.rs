use std::sync::Arc;

use twilight_model::gateway::payload::incoming::{InteractionCreate, MessageCreate};

use crate::CommandMeta;
use rusty_core::Context;
use rusty_utils::pagination::{
    DEFAULT_TIMEOUT_SECS, PaginationInteractionValidation, PaginationModalSubmitValidation,
    build_paginated_list_view, clamp_page, open_jump_modal_from_token, parse_one_based_page,
    resolve_modal_target_page, send_paginated_message, total_pages,
    update_paginated_interaction_message, validate_interaction_for_command,
    validate_jump_modal_for_command,
};

pub const META: CommandMeta = CommandMeta {
    name: "pagetest",
    desc: "Test embed pagination behavior.",
    category: "utility",
    usage: "!pagetest [page]",
};

// TODO: Remove this temporary command after pagination verification is complete.

const ITEMS_PER_PAGE: usize = 5;

/// Temporary pagination test command.
///
/// Purpose:
/// - verify embed rendering and pagination interactions.
///
/// Inputs:
/// - optional page number: `!pagetest [page]`.
///
/// Error behavior:
/// - returns usage text on invalid page input.
/// - returns bounds text when the requested page is out of range.
pub async fn run(ctx: Context, msg: Box<MessageCreate>, arg1: Option<&str>) -> anyhow::Result<()> {
    let http = &ctx.http;
    let items = build_test_items();
    let total = total_pages(items.len(), ITEMS_PER_PAGE);

    let Some(requested_page) = parse_one_based_page(arg1) else {
        let usage = format!("Usage: `{}` (page starts at 1)", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    if requested_page > total {
        let out = format!(
            "Page {} does not exist. Available pages: 1-{}.",
            requested_page, total
        );
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let (embed, components) = build_paginated_list_view(
        "pagetest",
        "Pagination Test",
        &items,
        requested_page,
        ITEMS_PER_PAGE,
        msg.author.id.get(),
        DEFAULT_TIMEOUT_SECS,
    )?;

    send_paginated_message(
        Arc::clone(&ctx.http),
        msg.channel_id,
        embed,
        components,
        total,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(())
}

/// Handle pagination button presses for the temporary `pagetest` command.
pub async fn handle_pagination_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, token) =
        match validate_interaction_for_command(http, &interaction, "pagetest").await? {
            PaginationInteractionValidation::NotForCommand => return Ok(false),
            PaginationInteractionValidation::HandledInvalid => return Ok(true),
            PaginationInteractionValidation::Valid {
                actor_user_id,
                token,
            } => (actor_user_id, token),
        };

    let items = build_test_items();
    let total = total_pages(items.len(), ITEMS_PER_PAGE);

    if token.action == "jump" {
        open_jump_modal_from_token(http, &interaction, &token, total).await?;
        return Ok(true);
    }

    let target_page = clamp_page(token.page, total);

    let (embed, components) = build_paginated_list_view(
        "pagetest",
        "Pagination Test",
        &items,
        target_page,
        ITEMS_PER_PAGE,
        actor_id,
        DEFAULT_TIMEOUT_SECS,
    )?;

    update_paginated_interaction_message(
        Arc::clone(&ctx.http),
        &interaction,
        embed,
        components,
        total,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(true)
}

/// Handle jump-modal submit interactions for the temporary `pagetest` command.
pub async fn handle_pagination_modal_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, entered_page, total_pages_hint) =
        match validate_jump_modal_for_command(http, &interaction, "pagetest").await? {
            PaginationModalSubmitValidation::NotForCommand => return Ok(false),
            PaginationModalSubmitValidation::HandledInvalid => return Ok(true),
            PaginationModalSubmitValidation::Valid {
                actor_user_id,
                requested_page,
                total_pages_hint,
                ..
            } => (actor_user_id, requested_page, total_pages_hint),
        };

    let items = build_test_items();
    let total: usize = total_pages(items.len(), ITEMS_PER_PAGE);
    let target_page = resolve_modal_target_page(entered_page, total, total_pages_hint);

    let (embed, components) = build_paginated_list_view(
        "pagetest",
        "Pagination Test",
        &items,
        target_page,
        ITEMS_PER_PAGE,
        actor_id,
        DEFAULT_TIMEOUT_SECS,
    )?;

    update_paginated_interaction_message(
        Arc::clone(&ctx.http),
        &interaction,
        embed,
        components,
        total,
        DEFAULT_TIMEOUT_SECS,
    )
    .await?;

    Ok(true)
}

fn build_test_items() -> Vec<String> {
    (1..=24)
        .map(|index| format!("Sample pagination item #{index}"))
        .collect()
}
