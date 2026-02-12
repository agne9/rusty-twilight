use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::commands::CommandMeta;

pub const META: CommandMeta = CommandMeta {
    name: "universe",
    desc: "The answer to the universe.",
    category: "utility",
};

pub async fn run(http: Arc<Client>, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    http.create_message(msg.channel_id)
        .content("The answer to the universe is 67 😹")
        .await?;

    Ok(())
}
