use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::{COMMANDS, CommandMeta};
use rusty_core::Context;

pub const META: CommandMeta = CommandMeta {
    name: "usage",
    desc: "Show usage syntax for a specific command.",
    category: "utility",
    usage: "!usage <command>",
};

/// Show usage for a specific command.
///
/// Purpose:
/// - provide a focused syntax lookup per command.
///
/// Inputs:
/// - required command name: `!usage <command>`.
///
/// Error behavior:
/// - missing argument returns this command's usage.
/// - unknown command returns a short not-found message.
pub async fn run(ctx: Context, msg: Box<MessageCreate>, arg1: Option<&str>) -> anyhow::Result<()> {
    let http = &ctx.http;
    let Some(raw_name) = arg1 else {
        let usage = format!("Usage: `{}`", META.usage);
        http.create_message(msg.channel_id).content(&usage).await?;
        return Ok(());
    };

    let lookup = raw_name.trim().trim_start_matches('!').to_ascii_lowercase();

    let Some(command) = COMMANDS.iter().find(|command| command.name == lookup) else {
        let out = format!("Unknown command: `{}`", lookup);
        http.create_message(msg.channel_id).content(&out).await?;
        return Ok(());
    };

    let out = format!("Usage: `{}`", command.usage);
    http.create_message(msg.channel_id).content(&out).await?;

    Ok(())
}
