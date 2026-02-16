pub mod moderation;
pub mod utility;

use twilight_model::{
    application::interaction::InteractionData,
    gateway::payload::incoming::{InteractionCreate, MessageCreate},
};

use rusty_core::Context;
use rusty_utils::COMMAND_PREFIX;

#[derive(Clone, Copy)]
enum InteractionRoute {
    PermissionsButtons,
    HelpButtons,
    PagetestButtons,
    TerminateButtons,
    PermissionsModal,
    HelpModal,
    PagetestModal,
}

fn route_interaction(custom_id: &str) -> Option<InteractionRoute> {
    const ROUTES: [(&str, InteractionRoute); 7] = [
        ("pg:permissions:", InteractionRoute::PermissionsButtons),
        ("pg:help", InteractionRoute::HelpButtons),
        ("pg:pagetest:", InteractionRoute::PagetestButtons),
        ("terminate:", InteractionRoute::TerminateButtons),
        ("pgm:permissions:", InteractionRoute::PermissionsModal),
        ("pgm:help", InteractionRoute::HelpModal),
        ("pgm:pagetest:", InteractionRoute::PagetestModal),
    ];

    ROUTES
        .into_iter()
        .find_map(|(prefix, route)| custom_id.starts_with(prefix).then_some(route))
}

// Global command meta data
pub struct CommandMeta {
    pub name: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub usage: &'static str,
}

pub const COMMANDS: &[CommandMeta] = &[
    utility::ping::META,
    utility::universe::META,
    utility::help::META,
    utility::usage::META,
    utility::pagetest::META,
    moderation::ban::META,
    moderation::unban::META,
    moderation::kick::META,
    moderation::timeout::META,
    moderation::untimeout::META,
    moderation::warn::META,
    moderation::warnings::META,
    moderation::purge::META,
    moderation::permissions::META,
    moderation::terminate::META,
    // Add new commands here
];

pub async fn handle_message(ctx: Context, msg: Box<MessageCreate>) -> anyhow::Result<()> {
    if msg.author.bot {
        return Ok(());
    }

    let content_owned = msg.content.clone();
    let content = content_owned.trim();

    if !content.starts_with(COMMAND_PREFIX) {
        return Ok(());
    }

    let content = content.trim_start_matches(COMMAND_PREFIX).trim();
    let mut command_and_rest = content.splitn(2, char::is_whitespace);
    let cmd = command_and_rest.next().unwrap_or("").to_ascii_lowercase();
    let rest = command_and_rest
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let (arg1, arg_tail): (Option<String>, Option<String>) = match rest {
        Some(value) => {
            let mut args = value.splitn(2, char::is_whitespace);
            let first = args
                .next()
                .filter(|arg| !arg.is_empty())
                .map(ToOwned::to_owned);
            let tail = args
                .next()
                .map(str::trim)
                .filter(|remaining| !remaining.is_empty())
                .map(ToOwned::to_owned);

            (first, tail)
        }
        None => (None, None),
    };

    let arg1 = arg1.as_deref();
    let arg_tail = arg_tail.as_deref();

    match cmd.as_str() {
        "ping" => utility::ping::run(ctx.clone(), msg).await?,
        "universe" => utility::universe::run(ctx.clone(), msg).await?,
        "help" => utility::help::run(ctx.clone(), msg, arg1).await?,
        "usage" => utility::usage::run(ctx.clone(), msg, arg1).await?,
        "pagetest" => utility::pagetest::run(ctx.clone(), msg, arg1).await?,

        "ban" => moderation::ban::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "unban" => moderation::unban::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "kick" => moderation::kick::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "timeout" => moderation::timeout::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "untimeout" => moderation::untimeout::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "warn" => moderation::warn::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "warnings" => moderation::warnings::run(ctx.clone(), msg, arg1, arg_tail).await?,
        "permissions" => moderation::permissions::run(ctx.clone(), msg, arg1).await?,
        "purge" => moderation::purge::run(ctx.clone(), msg, arg1).await?,
        "terminate" => moderation::terminate::run(ctx.clone(), msg, arg1, arg_tail).await?,
        // Add new commands here
        _ => {}
    }

    Ok(())
}

pub async fn handle_interaction(
    ctx: Context,
    interaction: Box<InteractionCreate>,
) -> anyhow::Result<()> {
    let custom_id = match interaction.data.as_ref() {
        Some(InteractionData::MessageComponent(data)) => data.custom_id.clone(),
        Some(InteractionData::ModalSubmit(data)) => data.custom_id.clone(),
        _ => return Ok(()),
    };

    let Some(route) = route_interaction(&custom_id) else {
        return Ok(());
    };

    match route {
        InteractionRoute::PermissionsButtons => {
            let _handled =
                moderation::permissions::handle_pagination_interaction(ctx.clone(), interaction)
                    .await?;
        }
        InteractionRoute::HelpButtons => {
            let _handled =
                utility::help::handle_pagination_interaction(ctx.clone(), interaction).await?;
        }
        InteractionRoute::PagetestButtons => {
            let _handled =
                utility::pagetest::handle_pagination_interaction(ctx.clone(), interaction).await?;
        }
        InteractionRoute::TerminateButtons => {
            let _handled =
                moderation::terminate::handle_interaction(ctx.clone(), interaction).await?;
        }
        InteractionRoute::PermissionsModal => {
            let _handled = moderation::permissions::handle_pagination_modal_interaction(
                ctx.clone(),
                interaction,
            )
            .await?;
        }
        InteractionRoute::HelpModal => {
            let _handled =
                utility::help::handle_pagination_modal_interaction(ctx.clone(), interaction)
                    .await?;
        }
        InteractionRoute::PagetestModal => {
            let _handled =
                utility::pagetest::handle_pagination_modal_interaction(ctx.clone(), interaction)
                    .await?;
        }
    }

    Ok(())
}
