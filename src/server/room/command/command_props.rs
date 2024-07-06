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
