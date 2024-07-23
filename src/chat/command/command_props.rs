use std::fmt::Debug;

pub trait CommandProps: Debug {
    fn cmd(&self) -> &str;
    fn args(&self) -> &str;
    fn help(&self) -> &str;
    fn is_op(&self) -> bool;

    fn is_visible(&self) -> bool {
        !self.help().is_empty()
    }

    fn has_prefix(&self, prefix: &str) -> bool {
        self.cmd().starts_with(prefix)
    }
}

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn is_visible_return_true_when_help_is_not_empty() {
        let command = MockCommand::new("test_cmd", "test_args", "test_help", true);
        assert!(command.is_visible());
    }

    #[test]
    fn is_visible_return_false_when_help_is_empty() {
        let command = MockCommand::new("test_cmd", "test_args", "", true);
        assert!(!command.is_visible());
    }

    #[test]
    fn has_prefix_return_true_when_cmd_starts_with_prefix() {
        let command = MockCommand::new("test_cmd", "test_args", "test_help", true);
        assert!(command.has_prefix("test"));
    }

    #[test]
    fn has_prefix_return_false_when_cmd_does_not_start_with_prefix() {
        let command = MockCommand::new("test_cmd", "test_args", "test_help", true);
        assert!(!command.has_prefix("cmd"));
    }

    #[derive(Debug)]
    struct MockCommand {
        cmd: &'static str,
        args: &'static str,
        help: &'static str,
        is_op: bool,
    }

    impl MockCommand {
        fn new(cmd: &'static str, args: &'static str, help: &'static str, is_op: bool) -> Self {
            Self {
                cmd,
                args,
                help,
                is_op,
            }
        }
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

        fn is_op(&self) -> bool {
            self.is_op
        }
    }
}
