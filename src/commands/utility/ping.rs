use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::commands::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "ping",
    desc: "Replies with Pong!",
    category: "utility",
};

pub async fn run(http: Arc<Client>, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    http.create_message(msg.channel_id).content("Pong!").await?;

    Ok(())
}
