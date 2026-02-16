/// Generic embed builders shared across commands.
pub mod embed;
/// Single source of truth for the message-command prefix.
pub const COMMAND_PREFIX: char = '!';
/// Shared pagination helpers and interaction utilities.
pub mod pagination;
/// Pure parser helpers.
pub mod parse;
/// Permission helper utilities.
pub mod permissions;
