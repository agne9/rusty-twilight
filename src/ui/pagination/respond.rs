//! Shared interaction response helpers for pagination flows.

use twilight_http::Client;
use twilight_model::{
    channel::message::{MessageFlags, embed::Embed},
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use twilight_model::channel::message::component::Component;

/// Generic message shown when a pagination interaction belongs to another user.
pub const PAGINATION_WRONG_USER_MESSAGE: &str = "This pagination session belongs to another user.";
/// Generic message shown when a pagination interaction has expired.
pub const PAGINATION_EXPIRED_MESSAGE: &str =
    "This pagination session expired. Run the command again.";
/// Generic message shown when pagination interaction payload is invalid.
pub const PAGINATION_INVALID_MESSAGE: &str = "Invalid pagination interaction.";

/// Respond to a component interaction with an in-place message update.
pub async fn respond_update_message(
    http: &Client,
    interaction: &InteractionCreate,
    embeds: &[Embed],
    components: &[Component],
) -> anyhow::Result<()> {
    let response = InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(
            InteractionResponseDataBuilder::new()
                .embeds(embeds.to_vec())
                .components(components.to_vec())
                .build(),
        ),
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

/// Respond to a component interaction with an ephemeral message.
pub async fn respond_ephemeral_message(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
) -> anyhow::Result<()> {
    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .content(content)
                .flags(MessageFlags::EPHEMERAL)
                .build(),
        ),
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

/// Respond with the standard wrong-owner pagination message.
pub async fn respond_wrong_user(
    http: &Client,
    interaction: &InteractionCreate,
) -> anyhow::Result<()> {
    respond_ephemeral_message(http, interaction, PAGINATION_WRONG_USER_MESSAGE).await
}

/// Respond with the standard expired pagination message.
pub async fn respond_expired(http: &Client, interaction: &InteractionCreate) -> anyhow::Result<()> {
    respond_ephemeral_message(http, interaction, PAGINATION_EXPIRED_MESSAGE).await
}

/// Respond with the standard invalid pagination message.
pub async fn respond_invalid(http: &Client, interaction: &InteractionCreate) -> anyhow::Result<()> {
    respond_ephemeral_message(http, interaction, PAGINATION_INVALID_MESSAGE).await
}
