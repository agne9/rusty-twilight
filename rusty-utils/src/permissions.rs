use twilight_http::Client;
use twilight_model::{gateway::payload::incoming::MessageCreate, guild::Permissions};

/// Convert a permission bitset into a sorted display list.
///
/// If `ADMINISTRATOR` is present, only `ADMINISTRATOR` is returned because
/// it implicitly grants all permissions.
pub fn permission_names(perms: Permissions) -> Vec<String> {
    if perms.contains(Permissions::ADMINISTRATOR) {
        return vec!["ADMINISTRATOR".to_owned()];
    }

    let mut names: Vec<String> = perms
        .iter_names()
        .map(|(name, _flag)| name.to_owned())
        .collect();
    names.sort_unstable();
    names
}

/// Resolve the invoking author's effective guild permissions for a message command.
///
/// Returns `Ok(None)` when the message is not from a guild context.
pub async fn resolve_message_author_permissions(
    http: &Client,
    msg: &MessageCreate,
) -> anyhow::Result<Option<Permissions>> {
    if let Some(perms) = msg.member.as_ref().and_then(|m| m.permissions) {
        return Ok(Some(perms));
    }

    let Some(guild_id) = msg.guild_id else {
        return Ok(None);
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

    Ok(Some(resolved))
}

/// Check whether the message author has a required permission (or administrator).
///
/// Returns `Ok(false)` when the message is outside a guild context.
pub async fn has_message_permission(
    http: &Client,
    msg: &MessageCreate,
    required: Permissions,
) -> anyhow::Result<bool> {
    let Some(perms) = resolve_message_author_permissions(http, msg).await? else {
        return Ok(false);
    };

    Ok(perms.contains(Permissions::ADMINISTRATOR) || perms.contains(required))
}
