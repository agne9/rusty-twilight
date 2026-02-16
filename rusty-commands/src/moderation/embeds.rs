use twilight_http::Client;
use twilight_model::{channel::message::embed::Embed, id::Id, id::marker::UserMarker};
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, ImageSource};

use rusty_database::model::warnings::WarningEntry;
use rusty_utils::embed::DEFAULT_EMBED_COLOR;

/// Build a moderation action-result embed.
///
/// This is a pure view/template helper and does not perform HTTP requests.
#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Resolve a moderation target profile for display in embeds.
pub async fn fetch_target_profile(http: &Client, user_id: Id<UserMarker>) -> TargetProfile {
    let user = match http.user(user_id).await {
        Ok(response) => match response.model().await {
            Ok(model) => model,
            Err(_) => {
                return TargetProfile {
                    display_name: format!("User {}", user_id.get()),
                    avatar_url: None,
                };
            }
        },
        Err(_) => {
            return TargetProfile {
                display_name: format!("User {}", user_id.get()),
                avatar_url: None,
            };
        }
    };

    let display_name = user.global_name.unwrap_or(user.name);
    let avatar_url = Some(match user.avatar {
        Some(avatar) => format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=128",
            user_id.get(),
            avatar
        ),
        None => {
            let default_avatar_index = (user_id.get() >> 22) % 6;
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                default_avatar_index
            )
        }
    });

    TargetProfile {
        display_name,
        avatar_url,
    }
}

pub fn moderation_action_embed(
    target_profile: &TargetProfile,
    target_user_id: Id<UserMarker>,
    action_past_tense: &str,
    reason: Option<&str>,
    duration: Option<&str>,
) -> anyhow::Result<Embed> {
    let reason = reason
        .unwrap_or("No reason provided")
        .replace('@', "@\u{200B}");

    let description = match duration {
        Some(duration) => format!(
            "Target: <@{}>\nReason: {}\nDuration: {}",
            target_user_id.get(),
            reason,
            duration
        ),
        None => format!("Target: <@{}>\nReason: {}", target_user_id.get(), reason),
    };

    let builder = EmbedBuilder::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    let builder = match target_profile.avatar_url.as_deref() {
        Some(url) => {
            let icon = ImageSource::url(url.to_owned())?;
            let author = EmbedAuthorBuilder::new(format!(
                "{} has been {}",
                target_profile.display_name, action_past_tense
            ))
            .icon_url(icon)
            .build();
            builder.author(author)
        }
        None => builder.title(format!(
            "{} has been {}",
            target_profile.display_name, action_past_tense
        )),
    };

    Ok(builder.validate()?.build())
}

pub fn usage_message(usage: &str) -> String {
    format!("Usage: `{usage}`")
}

pub fn guild_only_message() -> &'static str {
    "This command only works in servers."
}

pub fn permission_denied_message() -> &'static str {
    "You are not permitted to use this command."
}

pub fn moderation_permission_combo_denied_message(required: &str) -> String {
    format!("You need {required} permissions to use this command.")
}

pub fn moderation_self_action_message(action: &str) -> String {
    format!("You can't {action} yourself.")
}

pub fn moderation_invalid_interaction_message(action: &str) -> String {
    format!("Invalid {action} interaction.")
}

pub fn warnings_window_label_days(days: u64) -> String {
    format!("last {} day(s)", days)
}

pub fn warnings_overview_embed(
    target_profile: &TargetProfile,
    window_label: &str,
    entries: &[WarningEntry],
) -> anyhow::Result<Embed> {
    let count = entries.len();
    let mut description = format!("Total warnings in {}: **{}**\n\n", window_label, count);

    if entries.is_empty() {
        description.push_str("No warnings in this period.");
    } else {
        let start = entries.len().saturating_sub(5);
        for (index, entry) in entries.iter().enumerate().skip(start) {
            let line = format!(
                "#{idx} • <t:{ts}:F> • by <@{mod_id}>\nReason: {reason}\n\n",
                idx = index + 1,
                ts = entry.warned_at,
                mod_id = entry.moderator_id,
                reason = sanitize_reason(&entry.reason)
            );
            description.push_str(&line);
        }
    }

    let title = format!("Warnings for {}", target_profile.display_name);
    let builder = EmbedBuilder::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    let builder = match target_profile.avatar_url.as_deref() {
        Some(url) => {
            let icon = ImageSource::url(url.to_owned())?;
            let author = EmbedAuthorBuilder::new(title).icon_url(icon).build();
            builder.author(author)
        }
        None => builder.title(title),
    };

    Ok(builder.validate()?.build())
}

fn sanitize_reason(reason: &str) -> String {
    reason.replace('@', "@\u{200B}")
}
