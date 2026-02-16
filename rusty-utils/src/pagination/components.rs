//! Pagination UI component builders (previous/next buttons).

use std::time::{SystemTime, UNIX_EPOCH};

use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};

use super::token::build_custom_id;

/// Build previous/next button components for a paginated message.
pub fn build_nav_components(
    command: &str,
    current_page: usize,
    total_pages: usize,
    user_id: u64,
    timeout_secs: u64,
) -> Vec<Component> {
    if total_pages <= 1 {
        return vec![];
    }

    let expires_at = now_unix_secs().saturating_add(timeout_secs);

    let prev_page = if current_page > 1 {
        current_page - 1
    } else {
        current_page
    };

    let next_page = if current_page < total_pages {
        current_page + 1
    } else {
        current_page
    };

    let prev_button = Button {
        id: None,
        custom_id: Some(build_custom_id(
            command,
            "prev",
            prev_page,
            total_pages,
            user_id,
            expires_at,
        )),
        disabled: current_page <= 1,
        emoji: None,
        label: Some("◀ Prev".to_owned()),
        style: ButtonStyle::Secondary,
        url: None,
        sku_id: None,
    };

    let next_button = Button {
        id: None,
        custom_id: Some(build_custom_id(
            command,
            "next",
            next_page,
            total_pages,
            user_id,
            expires_at,
        )),
        disabled: current_page >= total_pages,
        emoji: None,
        label: Some("Next ▶".to_owned()),
        style: ButtonStyle::Secondary,
        url: None,
        sku_id: None,
    };

    let jump_button = Button {
        id: None,
        custom_id: Some(build_custom_id(
            command,
            "jump",
            current_page,
            total_pages,
            user_id,
            expires_at,
        )),
        disabled: false,
        emoji: None,
        label: Some("*".to_owned()),
        style: ButtonStyle::Secondary,
        url: None,
        sku_id: None,
    };

    vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![
            Component::Button(prev_button),
            Component::Button(jump_button),
            Component::Button(next_button),
        ],
    })]
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
