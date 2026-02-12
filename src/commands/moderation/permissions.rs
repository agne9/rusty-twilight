use std::sync::Arc;
use twilight_http::Client;
use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

use crate::commands::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "permissions",
    desc: "Display your server permissions.",
    category: "moderation",
};

pub async fn run(http: Arc<Client>, msg: Box<MessageCreate>) -> anyhow::Result<()> {
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

    let names: Vec<&str> = perms.iter_names().map(|(name, _flag)| name).collect();

    let out = if names.is_empty() {
        "You have no permissions set.".to_string()
    } else {
        format!("Your permissions:\n- {}", names.join("\n- "))
    };

    http.create_message(msg.channel_id).content(&out).await?;

    Ok(())
}
