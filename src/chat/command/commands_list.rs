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

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn format_commands_correctly() {
        let commands = vec![
            MockCommand {
                cmd: "cmd1",
                args: "args1",
                help: "help1",
                is_visible: true,
                is_op: false,
            },
            MockCommand {
                cmd: "cmd2",
                args: "args2",
                help: "help2",
                is_visible: true,
                is_op: false,
            },
        ];
        let expected =
            "cmd1       args1                help1\n\rcmd2       args2                help2";
        assert_eq!(format_commands(&commands), expected);
    }

    #[test]
    fn sort_by_command_length() {
        let mut commands = vec![
            MockCommand {
                cmd: "cmd1",
                args: "",
                help: "",
                is_visible: true,
                is_op: false,
            },
            MockCommand {
                cmd: "cmd",
                args: "",
                help: "",
                is_visible: true,
                is_op: false,
            },
        ];
        commands.sort_by(order_asc);
        assert_eq!(commands[0].cmd, "cmd");
        assert_eq!(commands[1].cmd, "cmd1");
    }

    #[test]
    fn filter_visible_commands() {
        let commands = vec![
            MockCommand {
                cmd: "cmd1",
                args: "",
                help: "",
                is_visible: true,
                is_op: false,
            },
            MockCommand {
                cmd: "cmd2",
                args: "",
                help: "",
                is_visible: false,
                is_op: false,
            },
        ];
        let filtered: Vec<_> = commands.into_iter().filter(filter_to_display).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].cmd, "cmd1");
    }

    #[test]
    fn filter_op_commands() {
        let commands = vec![
            MockCommand {
                cmd: "cmd1",
                args: "",
                help: "",
                is_visible: true,
                is_op: true,
            },
            MockCommand {
                cmd: "cmd2",
                args: "",
                help: "",
                is_visible: true,
                is_op: false,
            },
        ];
        let filtered: Vec<_> = commands.into_iter().filter(filter_op).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].cmd, "cmd1");
    }

    #[test]
    fn filter_noop_commands() {
        let commands = vec![
            MockCommand {
                cmd: "cmd1",
                args: "",
                help: "",
                is_visible: true,
                is_op: true,
            },
            MockCommand {
                cmd: "cmd2",
                args: "",
                help: "",
                is_visible: true,
                is_op: false,
            },
        ];
        let filtered: Vec<_> = commands.into_iter().filter(filter_noop).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].cmd, "cmd2");
    }

    #[derive(Debug, Clone)]
    struct MockCommand {
        cmd: &'static str,
        args: &'static str,
        help: &'static str,
        is_visible: bool,
        is_op: bool,
    }

    impl CommandProps for MockCommand {
        fn cmd(&self) -> &str {
            self.cmd
        }

        fn args(&self) -> &str {
            self.args
        }

        fn help(&self) -> &str {
            self.help
        }

        fn is_visible(&self) -> bool {
            self.is_visible
        }

        fn is_op(&self) -> bool {
            self.is_op
        }
    }
}
