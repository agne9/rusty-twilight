//! Stable facade for pagination helpers used by command handlers.

/// Default timeout for button-based pagination sessions.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

mod components;
pub mod interaction;
mod page;
pub mod respond;
pub mod token;
mod view;

pub use interaction::{
    PaginationInteractionValidation, PaginationModalSubmitValidation,
    validate_interaction_for_command, validate_interaction_for_command_prefix,
    validate_jump_modal_for_command, validate_jump_modal_for_command_prefix,
};
pub use page::{
    clamp_page, page_window, parse_one_based_page, resolve_modal_target_page, total_pages,
};
pub use respond::{
    open_jump_modal_from_token, respond_ephemeral_message, send_paginated_message,
    update_paginated_interaction_message,
};
pub use view::{
    build_paginated_list_view, build_paginated_view, build_paginated_view_with_footer_note,
};
