use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::CommandMeta;
use rusty_core::Context;

pub const META: CommandMeta = CommandMeta {
    name: "universe",
    desc: "The answer to the universe.",
    category: "utility",
    usage: "!universe",
};

/// Send the universe easter-egg response.
pub async fn run(ctx: Context, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    let http = &ctx.http;
    http.create_message(msg.channel_id)
        .content("The answer to the universe is 67 😹")
        .await?;

    Ok(())
}
