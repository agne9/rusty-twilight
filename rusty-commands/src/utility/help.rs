use std::sync::Arc;
use twilight_model::gateway::payload::incoming::{InteractionCreate, MessageCreate};

use crate::utility::embeds::{
    grouped_help_description, no_commands_message, page_out_of_range_message,
    unknown_category_message,
};
use crate::{COMMANDS, CommandMeta};
use rusty_core::Context;
use rusty_utils::pagination::{
    DEFAULT_TIMEOUT_SECS, PaginationInteractionValidation, PaginationModalSubmitValidation,
    build_paginated_view, build_paginated_view_with_footer_note, clamp_page,
    open_jump_modal_from_token, page_window, resolve_modal_target_page, send_paginated_message,
    total_pages, update_paginated_interaction_message, validate_interaction_for_command_prefix,
    validate_jump_modal_for_command_prefix,
};

pub const META: CommandMeta = CommandMeta {
    name: "help",
    desc: "Lists out all available commands.",
    category: "utility",
    usage: "!help [page|category]",
};

const HELP_COMMANDS_PER_PAGE: usize = 20;

/// Render the command catalog, optionally filtered by category or page.
pub async fn run(ctx: Context, msg: Box<MessageCreate>, arg1: Option<&str>) -> anyhow::Result<()> {
    let http = &ctx.http;
    let parsed_page = arg1.and_then(|raw| raw.parse::<usize>().ok().filter(|page| *page >= 1));
    let category = match (arg1, parsed_page) {
        (Some(raw), None) => Some(raw),
        _ => None,
    };

    let mut categories: Vec<&str> = COMMANDS.iter().map(|c| c.category).collect();
    categories.sort_unstable();
    categories.dedup();

    if let Some(wanted_category) = category
        && !categories.contains(&wanted_category)
    {
        let out = unknown_category_message(wanted_category, &categories);
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let commands = sorted_commands(category);
    if commands.is_empty() {
        let out = no_commands_message(category);
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let requested_page = parsed_page.unwrap_or(1);
    let total = total_pages(commands.len(), HELP_COMMANDS_PER_PAGE);

    if requested_page > total {
        let out = page_out_of_range_message(requested_page, total);
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let (start, end) = page_window(commands.len(), HELP_COMMANDS_PER_PAGE, requested_page);
    let description = grouped_help_description(&commands[start..end]);
    let pagination_command = help_pagination_command(category);
    let title = help_title();
    let footer_note = help_footer_note();

    let (embed, components) = match footer_note.as_deref() {
        Some(note) => build_paginated_view_with_footer_note(
            &pagination_command,
            &title,
            description,
            requested_page,
            total,
            msg.author.id.get(),
            DEFAULT_TIMEOUT_SECS,
            Some(note),
        )?,
        None => build_paginated_view(
            &pagination_command,
            &title,
            description,
            requested_page,
            total,
            msg.author.id.get(),
            DEFAULT_TIMEOUT_SECS,
        )?,
    };

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

/// Handle pagination button presses for the `help` command.
pub async fn handle_pagination_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, token) =
        match validate_interaction_for_command_prefix(http, &interaction, "help").await? {
            PaginationInteractionValidation::NotForCommand => return Ok(false),
            PaginationInteractionValidation::HandledInvalid => return Ok(true),
            PaginationInteractionValidation::Valid {
                actor_user_id,
                token,
            } => (actor_user_id, token),
        };

    let category = category_from_pagination_command(&token.command);
    let commands = sorted_commands(category.as_deref());
    if commands.is_empty() {
        return Ok(true);
    }

    let total = total_pages(commands.len(), HELP_COMMANDS_PER_PAGE);

    if token.action == "jump" {
        open_jump_modal_from_token(http, &interaction, &token, total).await?;
        return Ok(true);
    }

    let target_page = clamp_page(token.page, total);

    let (start, end) = page_window(commands.len(), HELP_COMMANDS_PER_PAGE, target_page);
    let description = grouped_help_description(&commands[start..end]);
    let title = help_title();
    let footer_note = help_footer_note();

    let (embed, components) = match footer_note.as_deref() {
        Some(note) => build_paginated_view_with_footer_note(
            &token.command,
            &title,
            description,
            target_page,
            total,
            actor_id,
            DEFAULT_TIMEOUT_SECS,
            Some(note),
        )?,
        None => build_paginated_view(
            &token.command,
            &title,
            description,
            target_page,
            total,
            actor_id,
            DEFAULT_TIMEOUT_SECS,
        )?,
    };

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

/// Handle jump-modal submit interactions for the `help` command.
pub async fn handle_pagination_modal_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let http = &ctx.http;
    let (actor_id, command, entered_page, total_pages_hint) =
        match validate_jump_modal_for_command_prefix(http, &interaction, "help").await? {
            PaginationModalSubmitValidation::NotForCommand => return Ok(false),
            PaginationModalSubmitValidation::HandledInvalid => return Ok(true),
            PaginationModalSubmitValidation::Valid {
                actor_user_id,
                command,
                requested_page,
                total_pages_hint,
            } => (actor_user_id, command, requested_page, total_pages_hint),
        };

    let category = category_from_pagination_command(&command);
    let commands = sorted_commands(category.as_deref());
    if commands.is_empty() {
        return Ok(true);
    }

    let total: usize = total_pages(commands.len(), HELP_COMMANDS_PER_PAGE);
    let target_page = resolve_modal_target_page(entered_page, total, total_pages_hint);

    let (start, end) = page_window(commands.len(), HELP_COMMANDS_PER_PAGE, target_page);
    let description = grouped_help_description(&commands[start..end]);
    let title = help_title();
    let footer_note = help_footer_note();

    let (embed, components) = match footer_note.as_deref() {
        Some(note) => build_paginated_view_with_footer_note(
            &command,
            &title,
            description,
            target_page,
            total,
            actor_id,
            DEFAULT_TIMEOUT_SECS,
            Some(note),
        )?,
        None => build_paginated_view(
            &command,
            &title,
            description,
            target_page,
            total,
            actor_id,
            DEFAULT_TIMEOUT_SECS,
        )?,
    };

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

fn help_pagination_command(category: Option<&str>) -> String {
    match category {
        Some(cat) => format!("help|{cat}"),
        None => "help".to_owned(),
    }
}

fn category_from_pagination_command(command: &str) -> Option<String> {
    command.strip_prefix("help|").map(ToOwned::to_owned)
}

fn help_title() -> String {
    "Available Commands".to_owned()
}

fn help_footer_note() -> Option<String> {
    None
}

fn sorted_commands(category: Option<&str>) -> Vec<&'static CommandMeta> {
    let mut filtered: Vec<&'static CommandMeta> = COMMANDS
        .iter()
        .filter(|cmd| match category {
            Some(wanted) => cmd.category == wanted,
            None => true,
        })
        .collect();

    filtered.sort_unstable_by(|left, right| {
        left.category
            .cmp(right.category)
            .then_with(|| left.name.cmp(right.name))
    });

    filtered
}
