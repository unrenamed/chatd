use strum::IntoEnumIterator;

use crate::utils;

use super::{Command, CommandProps, OplistCommand, WhitelistCommand};

lazy_static::lazy_static! {
    pub static ref CHAT_COMMANDS: Vec<Command> = {
        let mut commands = Command::iter().collect::<Vec<Command>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref OP_CHAT_COMMANDS: Vec<Command> = {
        let mut commands = Command::iter()
            .filter(filter_op)
            .collect::<Vec<Command>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref NOOP_CHAT_COMMANDS: Vec<Command> = {
        let mut commands = Command::iter()
            .filter(filter_noop)
            .collect::<Vec<Command>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref VISIBLE_OP_CHAT_COMMANDS: Vec<Command> = {
        let mut commands = Command::iter()
            .filter(filter_to_display)
            .filter(filter_op)
            .collect::<Vec<Command>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref VISIBLE_NOOP_CHAT_COMMANDS: Vec<Command> = {
        let mut commands = Command::iter()
            .filter(filter_to_display)
            .filter(filter_noop)
            .collect::<Vec<Command>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref WHITELIST_COMMANDS: Vec<WhitelistCommand> = {
        let mut commands = WhitelistCommand::iter().collect::<Vec<WhitelistCommand>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref VISIBLE_WHITELIST_COMMANDS: Vec<WhitelistCommand> = {
        let mut commands = WhitelistCommand::iter()
            .filter(filter_to_display)
            .collect::<Vec<WhitelistCommand>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref OPLIST_COMMANDS: Vec<OplistCommand> = {
        let mut commands = OplistCommand::iter().collect::<Vec<OplistCommand>>();
        commands.sort_by(order_asc);
        commands
    };
    pub static ref VISIBLE_OPLIST_COMMANDS: Vec<OplistCommand> = {
        let mut commands = OplistCommand::iter()
            .filter(filter_to_display)
            .collect::<Vec<OplistCommand>>();
        commands.sort_by(order_asc);
        commands
    };
}

pub fn format_commands<C: CommandProps>(commands: &Vec<C>) -> String {
    let count = commands.len();
    let mut result_vec = Vec::with_capacity(count);

    for (idx, cmd) in commands.iter().enumerate() {
        let formatted_command = format_command(cmd, idx == count - 1);
        result_vec.push(formatted_command);
    }

    result_vec.join("")
}

fn format_command<C: CommandProps>(cmd: &C, is_last: bool) -> String {
    format!(
        "{:<10} {:<20} {}{}",
        cmd.cmd(),
        cmd.args(),
        cmd.help(),
        if is_last { "" } else { utils::NEWLINE }
    )
}

fn order_asc<C: CommandProps + Clone>(a: &C, b: &C) -> std::cmp::Ordering {
    a.cmd().len().cmp(&b.cmd().len())
}

fn filter_to_display<C: CommandProps + Clone>(cmd: &C) -> bool {
    cmd.is_visible()
}

fn filter_op<C: CommandProps + Clone>(cmd: &C) -> bool {
    cmd.is_op()
}

fn filter_noop<C: CommandProps + Clone>(cmd: &C) -> bool {
    !cmd.is_op()
}
