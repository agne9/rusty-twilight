use std::env;
use std::sync::Arc;

use twilight_gateway::{EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client;
use twilight_model::gateway::event::Event;

use rustls::crypto::ring::default_provider;

mod commands;
mod services;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    default_provider()
        .install_default()
        .expect("failed to install rustls ring provider");

    // Load the .env file
    dotenvy::dotenv().ok();

    // Store Discord Bot Token
    let token = env::var("DISCORD_TOKEN")?;

    // Create a single shared HTTP Client
    let http = Arc::new(Client::new(token.clone()));

    // Declare which intents the bot has
    let intents = Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

    // A shard is one Gateway WebSocket connection to Discord
    // Declare how many shards we want to be running and input our token and intents
    let mut shard = Shard::new(ShardId::new(0, 1), token, intents);

    println!("Rusty is connecting...");

    // Our ears, listens for stuff to do
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(event) => event,
            Err(_) => continue,
        };

        match event {
            Event::Ready(_) => {
                println!("Rusty has successfully awoken!");
            }

            Event::MessageCreate(msg) => {
                commands::handle_message(Arc::clone(&http), msg).await?;
            }
            Event::InteractionCreate(interaction) => {
                commands::handle_interaction(Arc::clone(&http), interaction).await?;
            }
            _ => {} // Ignore unused events
        }
    }
    Ok(()) // Return Success, shutdown cleanly
}
