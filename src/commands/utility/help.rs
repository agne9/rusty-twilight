use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::{InteractionCreate, MessageCreate};

use crate::commands::{COMMANDS, CommandMeta};
use crate::ui::pagination::{
    DEFAULT_TIMEOUT_SECS, PaginationInteractionValidation, build_paginated_view,
    build_paginated_view_with_footer_note, clamp_page, page_window, respond_update_message,
    total_pages, validate_interaction_for_command_prefix,
};

pub const META: CommandMeta = CommandMeta {
    name: "help",
    desc: "Lists out all available commands.",
    category: "utility",
};

pub async fn run(
    http: Arc<Client>,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
) -> anyhow::Result<()> {
    let parsed_page = arg1.and_then(|raw| raw.parse::<usize>().ok().filter(|page| *page >= 1));
    let category = match (arg1, parsed_page) {
        (Some(raw), None) => Some(raw),
        _ => None,
    };

    let mut categories: Vec<&str> = COMMANDS.iter().map(|c| c.category).collect();
    categories.sort_unstable();
    categories.dedup();

    if let Some(wanted_category) = category
        && !categories
            .iter()
            .any(|known_category| *known_category == wanted_category)
    {
        let valid = categories
            .iter()
            .map(|category| display_category(category))
            .collect::<Vec<_>>()
            .join(", ");
        let out = format!(
            "Unknown category: {}\nValid categories: {}",
            display_category(wanted_category),
            valid
        );
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let commands = sorted_commands(category);
    if commands.is_empty() {
        let out = match category {
            Some(cat) => format!("No commands found in category: {}", display_category(cat)),
            None => {
                "No commands found at all. (This probably means something is broken)".to_owned()
            }
        };
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let requested_page = parsed_page.unwrap_or(1);
    let total = total_pages(commands.len(), HELP_PER_PAGE);

    if requested_page > total {
        let out = format!(
            "Page {} does not exist. Available pages: 1-{}.",
            requested_page, total
        );
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    }

    let (start, end) = page_window(commands.len(), HELP_PER_PAGE, requested_page);
    let description = grouped_help_description(&commands[start..end]);
    let pagination_command = help_pagination_command(category);
    let title = help_title(category);
    let footer_note = help_footer_note(category);

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

    http.create_message(msg.channel_id)
        .embeds(&[embed])
        .components(&components)
        .await?;

    Ok(())
}

const HELP_PER_PAGE: usize = 6;

/// Handle pagination button presses for the `help` command.
pub async fn handle_pagination_interaction(
    http: Arc<Client>,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<bool> {
    let (actor_id, token) =
        match validate_interaction_for_command_prefix(&http, &interaction, "help").await? {
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

    let total = total_pages(commands.len(), HELP_PER_PAGE);
    let target_page = clamp_page(token.page, total);

    let (start, end) = page_window(commands.len(), HELP_PER_PAGE, target_page);
    let description = grouped_help_description(&commands[start..end]);
    let title = help_title(category.as_deref());
    let footer_note = help_footer_note(category.as_deref());

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

    respond_update_message(&http, &interaction, &[embed], &components).await?;

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

fn help_title(category: Option<&str>) -> String {
    match category {
        Some(cat) => format!("Available Commands • {}", display_category(cat)),
        None => "Available Commands".to_owned(),
    }
}

fn help_footer_note(category: Option<&str>) -> Option<String> {
    category.map(|cat| format!("Category: {}", display_category(cat)))
}

fn display_category(category: &str) -> String {
    let mut chars = category.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
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

fn grouped_help_description(commands: &[&CommandMeta]) -> String {
    let mut out = String::new();
    let mut current_category: Option<&str> = None;

    for command in commands {
        if current_category != Some(command.category) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("**{}**\n", display_category(command.category)));
            current_category = Some(command.category);
        }

        out.push_str(&format!("• !{} - {}\n", command.name, command.desc));
    }

    if out.is_empty() {
        out.push_str("No commands available.");
    }

    out.trim_end().to_owned()
}
