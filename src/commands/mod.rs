pub mod moderation;
pub mod utility;

use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::MessageCreate;

// Global command meta data
pub struct CommandMeta {
    pub name: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
}

pub const COMMANDS: &[CommandMeta] = &[
    utility::ping::META,
    utility::universe::META,
    utility::help::META,
    moderation::purge::META,
    moderation::permissions::META,
    // Add new commands here
];

const PREFIX: char = '!'; // Command Prefix Character

pub async fn handle_message(http: Arc<Client>, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    if msg.author.bot {
        return Ok(());
    }

    if !msg.content.starts_with(PREFIX) {
        return Ok(());
    }

    let content = msg.content.to_ascii_lowercase();
    let mut parts = content.split_whitespace(); // Split msg into parts based on it's whitespaces
    let raw = parts.next().unwrap_or(""); // Take the first piece (command), or empty string if missing
    let cmd = raw.trim_start_matches('!'); // Remove prefix
    let arg1 = parts.next(); // Take first arg after command

    match cmd {
        "ping" => utility::ping::run(http, msg).await?,
        "universe" => utility::universe::run(http, msg).await?,
        "help" => utility::help::run(http, msg, arg1).await?,

        "permissions" => moderation::permissions::run(http, msg).await?,
        "purge" => moderation::purge::run(http, msg, arg1).await?,
        // Add new commands here
        _ => {}
    }

    Ok(())
}
