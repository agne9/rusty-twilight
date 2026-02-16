use twilight_http::Client;
use twilight_model::{
    channel::message::{
        component::{ActionRow, Button, ButtonStyle, Component},
        embed::Embed,
    },
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::pagination::{
    respond::respond_update_message, respond_ephemeral_message, respond_update_content_message,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationAction {
    Confirm,
    Decline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsedConfirmationAction {
    pub action: ConfirmationAction,
    pub requester_id: u64,
    pub target_id: u64,
    pub context_value: Option<u64>,
}

pub fn build_confirmation_custom_id(
    prefix: &str,
    action: ConfirmationAction,
    requester_id: u64,
    target_id: u64,
    context_value: Option<u64>,
) -> String {
    let action_segment = match action {
        ConfirmationAction::Confirm => "confirm",
        ConfirmationAction::Decline => "decline",
    };

    let context_raw = context_value.unwrap_or(0);
    format!("{prefix}{action_segment}:{requester_id}:{target_id}:{context_raw}")
}

pub fn build_confirmation_custom_ids(
    prefix: &str,
    requester_id: u64,
    target_id: u64,
    context_value: Option<u64>,
) -> (String, String) {
    (
        build_confirmation_custom_id(
            prefix,
            ConfirmationAction::Confirm,
            requester_id,
            target_id,
            context_value,
        ),
        build_confirmation_custom_id(
            prefix,
            ConfirmationAction::Decline,
            requester_id,
            target_id,
            context_value,
        ),
    )
}

pub fn build_confirmation_components(
    confirm_custom_id: String,
    decline_custom_id: String,
) -> Vec<Component> {
    vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![
            Component::Button(Button {
                id: None,
                custom_id: Some(confirm_custom_id),
                disabled: false,
                emoji: None,
                label: Some("Confirm".to_owned()),
                style: ButtonStyle::Danger,
                url: None,
                sku_id: None,
            }),
            Component::Button(Button {
                id: None,
                custom_id: Some(decline_custom_id),
                disabled: false,
                emoji: None,
                label: Some("Decline".to_owned()),
                style: ButtonStyle::Secondary,
                url: None,
                sku_id: None,
            }),
        ],
    })]
}

pub fn parse_confirmation_custom_id(
    custom_id: &str,
    prefix: &str,
) -> Option<ParsedConfirmationAction> {
    let raw = custom_id.strip_prefix(prefix)?;
    let mut parts = raw.split(':');

    let action = match parts.next()? {
        "confirm" => ConfirmationAction::Confirm,
        "decline" => ConfirmationAction::Decline,
        _ => return None,
    };

    let requester_id = parts.next()?.parse::<u64>().ok()?;
    let target_id = parts.next()?.parse::<u64>().ok()?;
    let context_raw = parts.next()?.parse::<u64>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    let context_value = (context_raw != 0).then_some(context_raw);

    Some(ParsedConfirmationAction {
        action,
        requester_id,
        target_id,
        context_value,
    })
}

pub async fn respond_update_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];
    respond_update_content_message(http, interaction, content, &empty_components).await
}

pub async fn respond_update_embed_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    embed: &Embed,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];
    respond_update_message(
        http,
        interaction,
        std::slice::from_ref(embed),
        &empty_components,
    )
    .await
}

pub async fn respond_update_content_embed_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
    embed: &Embed,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];
    let response = InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(
            InteractionResponseDataBuilder::new()
                .content(content)
                .embeds(vec![embed.clone()])
                .components(empty_components.to_vec())
                .build(),
        ),
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

pub async fn respond_ephemeral_notice(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
) -> anyhow::Result<()> {
    respond_ephemeral_message(http, interaction, content).await
}

pub async fn defer_component_update(
    http: &Client,
    interaction: &InteractionCreate,
) -> anyhow::Result<()> {
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredUpdateMessage,
        data: None,
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

pub async fn edit_original_response_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];

    http.interaction(interaction.application_id)
        .update_response(&interaction.token)
        .content(Some(content))
        .components(Some(&empty_components))
        .await?;

    Ok(())
}

pub async fn edit_original_response_embed_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    embed: &Embed,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];

    http.interaction(interaction.application_id)
        .update_response(&interaction.token)
        .content(None)
        .embeds(Some(std::slice::from_ref(embed)))
        .components(Some(&empty_components))
        .await?;

    Ok(())
}

pub async fn edit_original_response_content_embed_without_components(
    http: &Client,
    interaction: &InteractionCreate,
    content: &str,
    embed: &Embed,
) -> anyhow::Result<()> {
    let empty_components: [Component; 0] = [];

    http.interaction(interaction.application_id)
        .update_response(&interaction.token)
        .content(Some(content))
        .embeds(Some(std::slice::from_ref(embed)))
        .components(Some(&empty_components))
        .await?;

    Ok(())
}
