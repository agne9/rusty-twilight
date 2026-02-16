use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::CommandMeta;
use rusty_core::Context;

pub const META: CommandMeta = CommandMeta {
    name: "ping",
    desc: "Replies with Pong!",
    category: "utility",
    usage: "!ping",
};

/// Send a simple connectivity response.
pub async fn run(ctx: Context, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    let http = &ctx.http;
    http.create_message(msg.channel_id).content("Pong!").await?;

    Ok(())
}
