//! Embed and component composition helpers for paginated views.

use twilight_model::channel::message::component::Component;
use twilight_model::channel::message::embed::Embed;

use crate::embed::{build_paginated_embed, build_paginated_embed_with_footer_note};

use super::components::build_nav_components;
use super::page::{clamp_page, paginated_bulleted_description, total_pages};

/// Build a generic paginated list view (embed + navigation buttons).
pub fn build_paginated_list_view(
    command: &str,
    title: &str,
    items: &[String],
    page: usize,
    per_page: usize,
    owner_user_id: u64,
    timeout_secs: u64,
) -> anyhow::Result<(Embed, Vec<Component>)> {
    let total = total_pages(items.len(), per_page);
    let page = clamp_page(page, total);
    let description = paginated_bulleted_description(items, per_page, page);

    build_paginated_view(
        command,
        title,
        description,
        page,
        total,
        owner_user_id,
        timeout_secs,
    )
}

/// Build a paginated embed + navigation controls from a pre-rendered description.
pub fn build_paginated_view(
    command: &str,
    title: &str,
    description: String,
    page: usize,
    total_pages: usize,
    owner_user_id: u64,
    timeout_secs: u64,
) -> anyhow::Result<(Embed, Vec<Component>)> {
    build_paginated_view_with_footer_note(
        command,
        title,
        description,
        page,
        total_pages,
        owner_user_id,
        timeout_secs,
        None,
    )
}

/// Build a paginated embed + navigation controls with an optional footer note.
// Clippy note: this API mirrors pagination state fields passed from command handlers.
// Keeping arguments explicit here avoids temporary structs at every call site.
#[allow(clippy::too_many_arguments)]
pub fn build_paginated_view_with_footer_note(
    command: &str,
    title: &str,
    description: String,
    page: usize,
    total_pages: usize,
    owner_user_id: u64,
    timeout_secs: u64,
    footer_note: Option<&str>,
) -> anyhow::Result<(Embed, Vec<Component>)> {
    let page = clamp_page(page, total_pages);
    let total_pages = total_pages.max(1);

    let embed = match footer_note {
        Some(note) => build_paginated_embed_with_footer_note(
            title,
            description,
            page,
            total_pages,
            Some(note),
        )?,
        None => build_paginated_embed(title, description, page, total_pages)?,
    };

    let components = build_nav_components(command, page, total_pages, owner_user_id, timeout_secs);

    Ok((embed, components))
}
