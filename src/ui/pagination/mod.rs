//! Stable facade for pagination helpers used by command handlers.

/// Default timeout for button-based pagination sessions.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

mod components;
mod interaction;
mod page;
mod respond;
mod token;
mod view;

pub use interaction::{
    PaginationInteractionValidation, validate_interaction_for_command,
    validate_interaction_for_command_prefix,
};
pub use page::{clamp_page, page_window, parse_one_based_page, total_pages};
pub use respond::{respond_ephemeral_message, respond_update_message};
pub use view::{
    build_paginated_list_view, build_paginated_view, build_paginated_view_with_footer_note,
};
