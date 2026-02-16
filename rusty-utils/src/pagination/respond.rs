//! Shared interaction response helpers for pagination flows.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};

use twilight_http::Client;
use twilight_model::{
    channel::message::{
        MessageFlags,
        component::{ActionRow, Component, TextInput, TextInputStyle},
        embed::Embed,
    },
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker},
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::token::{PaginationToken, build_modal_custom_id};

type CleanupTaskMap = HashMap<u64, tokio::task::JoinHandle<()>>;

fn cleanup_tasks() -> &'static tokio::sync::Mutex<CleanupTaskMap> {
    static TASKS: OnceLock<tokio::sync::Mutex<CleanupTaskMap>> = OnceLock::new();
    TASKS.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

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

/// Open a modal allowing the user to jump to a page number.
#[allow(deprecated)]
pub async fn respond_jump_modal(
    http: &Client,
    interaction: &InteractionCreate,
    modal_custom_id: &str,
    title: &str,
    total_pages: usize,
) -> anyhow::Result<()> {
    let page_input = Component::TextInput(TextInput {
        id: None,
        custom_id: "page".to_owned(),
        label: Some("Page Number".to_owned()),
        max_length: Some(6),
        min_length: Some(1),
        placeholder: Some(format!("Enter a page from 1 to {total_pages}")),
        required: Some(true),
        style: TextInputStyle::Short,
        value: None,
    });

    let modal_components = vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![page_input],
    })];

    let response = InteractionResponse {
        kind: InteractionResponseType::Modal,
        data: Some(InteractionResponseData {
            components: Some(modal_components),
            custom_id: Some(modal_custom_id.to_owned()),
            title: Some(title.to_owned()),
            ..InteractionResponseData::default()
        }),
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

/// Open a jump modal from a validated pagination token.
pub async fn open_jump_modal_from_token(
    http: &Client,
    interaction: &InteractionCreate,
    token: &PaginationToken,
    total_pages: usize,
) -> anyhow::Result<()> {
    let modal_id =
        build_modal_custom_id(&token.command, total_pages, token.user_id, token.expires_at);
    respond_jump_modal(http, interaction, &modal_id, "Jump to Page", total_pages).await
}

/// Send a new paginated message and schedule component cleanup when needed.
pub async fn send_paginated_message(
    http: Arc<Client>,
    channel_id: Id<ChannelMarker>,
    embed: Embed,
    components: Vec<Component>,
    total_pages: usize,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let created_message = http
        .create_message(channel_id)
        .embeds(&[embed])
        .components(&components)
        .await?
        .model()
        .await?;

    if total_pages > 1 {
        schedule_component_cleanup(
            Arc::clone(&http),
            created_message.channel_id,
            created_message.id,
            timeout_secs,
        )
        .await;
    }

    Ok(())
}

/// Update an existing paginated interaction message and refresh cleanup timing.
pub async fn update_paginated_interaction_message(
    http: Arc<Client>,
    interaction: &InteractionCreate,
    embed: Embed,
    components: Vec<Component>,
    total_pages: usize,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    respond_update_message(&http, interaction, &[embed], &components).await?;

    if total_pages > 1
        && let Some(message) = interaction.message.as_ref()
    {
        schedule_component_cleanup(
            Arc::clone(&http),
            message.channel_id,
            message.id,
            timeout_secs,
        )
        .await;
    }

    Ok(())
}

/// Schedule removal of interactive components shortly before pagination timeout.
pub async fn schedule_component_cleanup(
    http: Arc<Client>,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
    timeout_secs: u64,
) {
    let delay_secs = timeout_secs.saturating_sub(1);
    let message_key = message_id.get();

    let mut tasks = cleanup_tasks().lock().await;
    if let Some(existing_task) = tasks.remove(&message_key) {
        existing_task.abort();
    }

    let cleanup_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;

        let empty_components: [Component; 0] = [];
        let _ = http
            .update_message(channel_id, message_id)
            .components(Some(&empty_components))
            .await;

        let mut tasks = cleanup_tasks().lock().await;
        tasks.remove(&message_key);
    });

    tasks.insert(message_key, cleanup_task);
}
