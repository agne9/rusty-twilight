use std::sync::Arc;
use twilight_http::Client;
use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use crate::commands::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "permissions",
    desc: "Display your server permissions (paginated).",
    category: "moderation",
};

const PERMISSIONS_PER_PAGE: usize = 10;

pub async fn run(
    http: Arc<Client>,
    msg: Box<MessageCreate>,
    arg1: Option<&str>,
) -> anyhow::Result<()> {
    let perms = if let Some(perms) = msg.member.as_ref().and_then(|m| m.permissions) {
        perms
    } else {
        let Some(guild_id) = msg.guild_id else {
            http.create_message(msg.channel_id)
                .content("This command only works in servers.")
                .await?;
            return Ok(());
        };

        let member = http
            .guild_member(guild_id, msg.author.id)
            .await?
            .model()
            .await?;

        let roles = http.roles(guild_id).await?.model().await?;

        let mut resolved = Permissions::empty();

        for role in roles {
            if role.id == guild_id.cast() || member.roles.contains(&role.id) {
                resolved |= role.permissions;
            }
        }

        resolved
    };

    if perms.is_empty() {
        http.create_message(msg.channel_id)
            .content("No permissions found for your member record.")
            .await?;
        return Ok(());
    }

    let names: Vec<&str> = if perms.contains(Permissions::ADMINISTRATOR) {
        vec!["ADMINISTRATOR"]
    } else {
        let mut names: Vec<&str> = perms.iter_names().map(|(name, _flag)| name).collect();
        names.sort_unstable();
        names
    };

    if names.is_empty() {
        http.create_message(msg.channel_id)
            .content("You have no permissions set.")
            .await?;
        return Ok(());
    }

    let total_pages = names.len().div_ceil(PERMISSIONS_PER_PAGE);
    let requested_page = match arg1 {
        Some(raw) => match raw.parse::<usize>() {
            Ok(page) if page >= 1 => page,
            _ => {
                http.create_message(msg.channel_id)
                    .content("Usage: !permissions [page], where page starts at 1.")
                    .await?;
                return Ok(());
            }
        },
        None => 1,
    };

    if requested_page > total_pages {
        let msg_out = format!(
            "Page {} does not exist. Available pages: 1-{}.",
            requested_page, total_pages
        );
        http.create_message(msg.channel_id)
            .content(&msg_out)
            .await?;
        return Ok(());
    }

    let start = (requested_page - 1) * PERMISSIONS_PER_PAGE;
    let end = (start + PERMISSIONS_PER_PAGE).min(names.len());
    let description = format!("- {}", names[start..end].join("\n- "));

    let footer = EmbedFooterBuilder::new(format!(
        "Page {}/{} • Use !permissions <page>",
        requested_page, total_pages
    ))
    .build();

    let embed = EmbedBuilder::new()
        .title("Your Server Permissions")
        .color(0x58_65_f2)
        .description(description)
        .footer(footer)
        .validate()?
        .build();

    http.create_message(msg.channel_id).embeds(&[embed]).await?;

    Ok(())
}
