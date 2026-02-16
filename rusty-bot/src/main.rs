use std::env;
use std::sync::Arc;

use tracing::{error, info};
use twilight_gateway::{EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client;
use twilight_model::gateway::event::Event;

use rustls::crypto::ring::default_provider;
use sqlx::postgres::PgPoolOptions;

use rusty_commands::{handle_interaction, handle_message};
use rusty_core::Context;
use rusty_database::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls ring provider"))?;

    // Load the .env file
    dotenvy::dotenv().ok();

    // Store Discord Bot Token
    let token = env::var("DISCORD_TOKEN")?;
    let database_url = env::var("DATABASE_URL")?;

    // Create a single shared HTTP Client
    let http = Arc::new(Client::new(token.clone()));
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("PostgreSQL connection established.");
    let db = Database::new(db_pool);
    let ctx = Context::new(Arc::clone(&http), db);

    // Declare which intents the bot has
    let intents = Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

    // A shard is one Gateway WebSocket connection to Discord
    // Declare how many shards we want to be running and input our token and intents
    let mut shard = Shard::new(ShardId::new(0, 1), token, intents);

    info!("Rusty is connecting...");

    // Our ears, listens for stuff to do
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(event) => event,
            Err(source) => {
                error!(?source, "gateway event stream error");
                continue;
            }
        };

        match event {
            Event::Ready(_) => {
                info!("Rusty has successfully awoken!");
            }

            Event::MessageCreate(msg) => {
                handle_message(ctx.clone(), msg).await?;
            }
            Event::InteractionCreate(interaction) => {
                handle_interaction(ctx.clone(), interaction).await?;
            }
            _ => {} // Ignore unused events
        }
    }
    Ok(()) // Return Success, shutdown cleanly
}
