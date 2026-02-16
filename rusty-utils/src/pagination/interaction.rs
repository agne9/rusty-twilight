//! Interaction validation orchestration for pagination component handlers.

use twilight_http::Client;
use twilight_model::gateway::payload::incoming::InteractionCreate;

use super::respond::{
    respond_ephemeral_message, respond_expired, respond_invalid, respond_wrong_user,
};
use super::token::{
    PaginationToken, PaginationValidationError, is_expired, parse_custom_id, parse_modal_custom_id,
    validate_custom_id,
};

/// Outcome when validating whether an interaction belongs to a pagination command.
#[derive(Debug, Clone)]
pub enum PaginationInteractionValidation {
    /// Interaction does not target the given command's pagination buttons.
    NotForCommand,
    /// Interaction was invalid and already acknowledged with a user-facing response.
    HandledInvalid,
    /// Interaction is valid and contains parsed pagination data.
    Valid {
        actor_user_id: u64,
        token: PaginationToken,
    },
}

/// Outcome when validating a pagination jump-modal submit interaction.
#[derive(Debug, Clone)]
pub enum PaginationModalSubmitValidation {
    /// Interaction does not target the given command's pagination modal.
    NotForCommand,
    /// Interaction was invalid and already acknowledged with a user-facing response.
    HandledInvalid,
    /// Interaction is valid and contains actor and requested page.
    Valid {
        actor_user_id: u64,
        command: String,
        requested_page: usize,
        total_pages_hint: usize,
    },
}

/// Validate whether an interaction is a pagination component for the given command.
///
/// Returns:
/// - `NotForCommand` when the interaction should be ignored by this handler,
/// - `HandledInvalid` when it was invalid and already acknowledged,
/// - `Valid` when parsing and validation succeeded.
pub async fn validate_interaction_for_command(
    http: &Client,
    interaction: &InteractionCreate,
    command: &str,
) -> anyhow::Result<PaginationInteractionValidation> {
    let Some(twilight_model::application::interaction::InteractionData::MessageComponent(
        component_data,
    )) = interaction.data.as_ref()
    else {
        return Ok(PaginationInteractionValidation::NotForCommand);
    };

    let expected_prefix = format!("pg:{command}:");
    if !component_data.custom_id.starts_with(&expected_prefix) {
        return Ok(PaginationInteractionValidation::NotForCommand);
    }

    let Some(actor_user_id) = interaction.author_id().map(|id| id.get()) else {
        respond_ephemeral_message(http, interaction, "Unable to determine interaction user.")
            .await?;
        return Ok(PaginationInteractionValidation::HandledInvalid);
    };

    match validate_custom_id(&component_data.custom_id, command, actor_user_id) {
        Ok(token) => Ok(PaginationInteractionValidation::Valid {
            actor_user_id,
            token,
        }),
        Err(PaginationValidationError::WrongUser) => {
            respond_wrong_user(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
        Err(PaginationValidationError::Expired) => {
            respond_expired(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
        Err(_) => {
            respond_invalid(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
    }
}

/// Validate whether an interaction is a pagination component for a command family.
///
/// This allows command keys like `help|utility`, while still requiring a shared
/// base prefix such as `help`.
pub async fn validate_interaction_for_command_prefix(
    http: &Client,
    interaction: &InteractionCreate,
    command_prefix: &str,
) -> anyhow::Result<PaginationInteractionValidation> {
    let Some(twilight_model::application::interaction::InteractionData::MessageComponent(
        component_data,
    )) = interaction.data.as_ref()
    else {
        return Ok(PaginationInteractionValidation::NotForCommand);
    };

    let raw_custom_id = &component_data.custom_id;
    let Some(token_preview) = parse_custom_id(raw_custom_id) else {
        return Ok(PaginationInteractionValidation::NotForCommand);
    };

    let belongs_to_prefix = token_preview.command == command_prefix
        || token_preview
            .command
            .starts_with(&format!("{command_prefix}|"));

    if !belongs_to_prefix {
        return Ok(PaginationInteractionValidation::NotForCommand);
    }

    let Some(actor_user_id) = interaction.author_id().map(|id| id.get()) else {
        respond_ephemeral_message(http, interaction, "Unable to determine interaction user.")
            .await?;
        return Ok(PaginationInteractionValidation::HandledInvalid);
    };

    match validate_custom_id(raw_custom_id, &token_preview.command, actor_user_id) {
        Ok(token) => Ok(PaginationInteractionValidation::Valid {
            actor_user_id,
            token,
        }),
        Err(PaginationValidationError::WrongUser) => {
            respond_wrong_user(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
        Err(PaginationValidationError::Expired) => {
            respond_expired(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
        Err(_) => {
            respond_invalid(http, interaction).await?;
            Ok(PaginationInteractionValidation::HandledInvalid)
        }
    }
}

/// Extract and parse the `page` text input value from a modal submit interaction.
pub fn parse_jump_modal_page(interaction: &InteractionCreate) -> Option<usize> {
    let twilight_model::application::interaction::InteractionData::ModalSubmit(modal_data) =
        interaction.data.as_ref()?
    else {
        return None;
    };

    for component in &modal_data.components {
        if let twilight_model::application::interaction::modal::ModalInteractionComponent::ActionRow(
            row,
        ) = component
        {
            for nested in &row.components {
                if let twilight_model::application::interaction::modal::ModalInteractionComponent::TextInput(
                    text_input,
                ) = nested
                    && text_input.custom_id == "page"
                {
                    return text_input
                        .value
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|page| *page >= 1);
                }
            }
        }
    }

    None
}

/// Validate a jump-modal submit interaction for an exact command.
pub async fn validate_jump_modal_for_command(
    http: &Client,
    interaction: &InteractionCreate,
    expected_command: &str,
) -> anyhow::Result<PaginationModalSubmitValidation> {
    validate_jump_modal_for_match(http, interaction, |command| command == expected_command).await
}

/// Validate a jump-modal submit interaction for a command family prefix.
pub async fn validate_jump_modal_for_command_prefix(
    http: &Client,
    interaction: &InteractionCreate,
    command_prefix: &str,
) -> anyhow::Result<PaginationModalSubmitValidation> {
    validate_jump_modal_for_match(http, interaction, |command| {
        command == command_prefix || command.starts_with(&format!("{command_prefix}|"))
    })
    .await
}

async fn validate_jump_modal_for_match(
    http: &Client,
    interaction: &InteractionCreate,
    matches: impl Fn(&str) -> bool,
) -> anyhow::Result<PaginationModalSubmitValidation> {
    let Some(modal_data) = interaction.data.as_ref().and_then(|data| {
        if let twilight_model::application::interaction::InteractionData::ModalSubmit(modal) = data
        {
            Some(modal)
        } else {
            None
        }
    }) else {
        return Ok(PaginationModalSubmitValidation::NotForCommand);
    };

    let Some(modal_token) = parse_modal_custom_id(&modal_data.custom_id) else {
        return Ok(PaginationModalSubmitValidation::NotForCommand);
    };

    if !matches(&modal_token.command) {
        return Ok(PaginationModalSubmitValidation::NotForCommand);
    }

    let Some(actor_user_id) = interaction.author_id().map(|id| id.get()) else {
        respond_ephemeral_message(http, interaction, "Unable to determine interaction user.")
            .await?;
        return Ok(PaginationModalSubmitValidation::HandledInvalid);
    };

    if modal_token.user_id != actor_user_id {
        respond_wrong_user(http, interaction).await?;
        return Ok(PaginationModalSubmitValidation::HandledInvalid);
    }

    if is_expired(modal_token.expires_at) {
        respond_expired(http, interaction).await?;
        return Ok(PaginationModalSubmitValidation::HandledInvalid);
    }

    let Some(requested_page) = parse_jump_modal_page(interaction) else {
        respond_ephemeral_message(http, interaction, "Please enter a valid page number.").await?;
        return Ok(PaginationModalSubmitValidation::HandledInvalid);
    };

    Ok(PaginationModalSubmitValidation::Valid {
        actor_user_id,
        command: modal_token.command,
        requested_page,
        total_pages_hint: modal_token.total_pages,
    })
}
