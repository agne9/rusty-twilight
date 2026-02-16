use tracing::error;
use twilight_http::Client;
use twilight_model::{
    channel::ChannelType,
    id::{
        Id,
        marker::{GuildMarker, MessageMarker, UserMarker},
    },
};
use tokio::time::{Duration, sleep};

use crate::time::now_unix_secs;

const BULK_DELETE_MAX_AGE_SECS: u64 = 14 * 24 * 60 * 60;
const BULK_DELETE_SAFETY_BUFFER_SECS: u64 = 60 * 60;
const HISTORY_PAGE_DELAY_MS: u64 = 1100;

/// Purge messages from a specific user across all text channels in a guild.
///
/// Returns the total number of messages successfully deleted.
pub async fn purge_user_globally(
    http: &Client,
    guild_id: Id<GuildMarker>,
    target_user_id: Id<UserMarker>,
    cutoff_secs: Option<u64>,
) -> anyhow::Result<u64> {
    let channels = http.guild_channels(guild_id).await?.model().await?;
    let mut deleted_count = 0_u64;
    let bulk_delete_cutoff = now_unix_secs()
        .saturating_sub(BULK_DELETE_MAX_AGE_SECS.saturating_sub(BULK_DELETE_SAFETY_BUFFER_SECS))
        as i64;

    for channel in channels {
        if !matches!(
            channel.kind,
            ChannelType::GuildText
                | ChannelType::GuildAnnouncement
                | ChannelType::PublicThread
                | ChannelType::PrivateThread
        ) {
            continue;
        }

        let channel_id = channel.id;
        let mut before: Option<Id<MessageMarker>> = None;

        loop {
            let response = match before {
                Some(before_id) => {
                    http.channel_messages(channel_id)
                        .before(before_id)
                        .limit(100)
                        .await
                }
                None => http.channel_messages(channel_id).limit(100).await,
            };

            let response = match response {
                Ok(response) => response,
                Err(_) => break,
            };

            let messages = match response.model().await {
                Ok(messages) => messages,
                Err(_) => break,
            };

            if messages.is_empty() {
                break;
            }

            before = messages.last().map(|message| message.id);

            let should_break_for_cutoff = cutoff_secs.is_some_and(|cutoff| {
                messages
                    .last()
                    .map(|last| last.timestamp.as_secs() < cutoff as i64)
                    .unwrap_or(false)
            });

            let mut bulk_candidate_ids: Vec<Id<MessageMarker>> = Vec::new();
            let mut single_delete_ids: Vec<Id<MessageMarker>> = Vec::new();

            for message in messages {
                if message.author.id != target_user_id {
                    continue;
                }

                if let Some(cutoff) = cutoff_secs
                    && message.timestamp.as_secs() < cutoff as i64
                {
                    continue;
                }

                if message.timestamp.as_secs() >= bulk_delete_cutoff {
                    bulk_candidate_ids.push(message.id);
                } else {
                    single_delete_ids.push(message.id);
                }
            }

            if !bulk_candidate_ids.is_empty() {
                for chunk in bulk_candidate_ids.chunks(100) {
                    if chunk.len() < 2 {
                        single_delete_ids.extend_from_slice(chunk);
                        continue;
                    }

                    match http.delete_messages(channel_id, chunk).await {
                        Ok(_) => {
                            deleted_count = deleted_count.saturating_add(chunk.len() as u64);
                        }
                        Err(source) => {
                            error!(
                                ?source,
                                channel_id = channel_id.get(),
                                count = chunk.len(),
                                "bulk delete failed, falling back to single delete"
                            );
                            single_delete_ids.extend_from_slice(chunk);
                        }
                    }
                }
            }

            for message_id in single_delete_ids {
                if http.delete_message(channel_id, message_id).await.is_ok() {
                    deleted_count = deleted_count.saturating_add(1);
                }
            }

            if should_break_for_cutoff {
                break;
            }

            sleep(Duration::from_millis(HISTORY_PAGE_DELAY_MS)).await;
        }
    }

    Ok(deleted_count)
}
