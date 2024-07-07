use strum::IntoEnumIterator;

use crate::utils;

use super::{Command, CommandProps, OplistCommand, WhitelistCommand};

lazy_static::lazy_static! {
    pub static ref CHAT_COMMANDS: Vec<Command> = Command::iter().collect();
    pub static ref WHITELIST_COMMANDS: Vec<WhitelistCommand> = WhitelistCommand::iter().collect();
    pub static ref OPLIST_COMMANDS: Vec<OplistCommand> = OplistCommand::iter().collect();
}

pub fn format_commands(commands: Vec<&impl CommandProps>) -> String {
    let count = commands.len();
    let mut result_vec = Vec::with_capacity(count);

    for (idx, cmd) in commands.iter().enumerate() {
        let formatted_command = format_command(*cmd, idx == count - 1);
        result_vec.push(formatted_command);
    }

    result_vec.join("")
}

fn format_command(cmd: &impl CommandProps, is_last: bool) -> String {
    format!(
        "{:<10} {:<20} {}{}",
        cmd.cmd(),
        cmd.args(),
        cmd.help(),
        if is_last { "" } else { utils::NEWLINE }
    )
}
