use std::sync::Arc;
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::commands::{CommandMeta, COMMANDS};

pub const META: CommandMeta = CommandMeta {
    name: "help",
    desc: "Lists out all available commands.",
    category: "utility",
};

pub async fn run(http: Arc<Client>, msg: Box<MessageCreate>, category: Option<&str>) -> anyhow::Result<()> {
    let mut out = String::from("**Available commands:**\n");

    // Collect categories from registry
    let mut categories: Vec<&str> = COMMANDS.iter().map(|c| c.category).collect();
    categories.sort_unstable();
    categories.dedup();

    let mut found = 0usize;

    // Check if category exists
    if let Some(wanted) = category {
        if !categories.iter().any(|c| *c == wanted) {
            let valid = categories.join(", ");
            let out = format!(
                "Unknown category: {}\nValid categories: {}", wanted, valid
            );

            http.create_message(msg.channel_id)
                .content(&out)
                .await?;

            return Ok(());
        }
    }

    for cmd in COMMANDS {
        // If category was provided, skip all other categories
        if let Some(wanted) = category {
            if cmd.category != wanted {
                continue;
            }
        }

        // Output example: "!help - Lists out all available commands. - (utility)"
        out.push_str(&format!("!{} - {} ({})\n", cmd.name, cmd.desc, cmd.category));

        found += 1;
    }

    if found == 0 {
        out = match category {
            Some(cat) => format!("No commands found in category: {}", cat),
            None => "No commands found at all. (This probably means something is broken)".to_string(),
        };
    }
    
    http.create_message(msg.channel_id)
    .content(&out)
    .await?;

    Ok(())
}
