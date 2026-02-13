//! Interaction validation orchestration for pagination component handlers.

use twilight_http::Client;
use twilight_model::gateway::payload::incoming::InteractionCreate;

use super::respond::{
    respond_ephemeral_message, respond_expired, respond_invalid, respond_wrong_user,
};
use super::token::{
    PaginationToken, PaginationValidationError, parse_custom_id, validate_custom_id,
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
