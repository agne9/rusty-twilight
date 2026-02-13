use twilight_model::channel::message::embed::Embed;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

/// Default embed color used across the bot UI.
pub const DEFAULT_EMBED_COLOR: u32 = 0x90_54_30;

/// Build a standard paginated embed with consistent styling.
pub fn build_paginated_embed(
    title: &str,
    description: impl Into<String>,
    page: usize,
    total_pages: usize,
) -> anyhow::Result<Embed> {
    build_paginated_embed_with_footer_note(title, description, page, total_pages, None)
}

/// Build a standard paginated embed with an optional footer suffix.
pub fn build_paginated_embed_with_footer_note(
    title: &str,
    description: impl Into<String>,
    page: usize,
    total_pages: usize,
    footer_note: Option<&str>,
) -> anyhow::Result<Embed> {
    let footer_text = match footer_note {
        Some(note) if !note.is_empty() => format!("Page {}/{} • {}", page, total_pages, note),
        _ => format!("Page {}/{}", page, total_pages),
    };

    let footer = EmbedFooterBuilder::new(footer_text).build();

    let embed = EmbedBuilder::new()
        .title(title)
        .color(DEFAULT_EMBED_COLOR)
        .description(description)
        .footer(footer)
        .validate()?
        .build();

    Ok(embed)
}
