mod autocomplete;
mod command_exec;
mod command_parse;
mod context;
mod emacs_key_exec;
mod env_parse;
mod handler;
mod input_rate_checker;
mod input_validator;

use autocomplete::Autocomplete;
use command_exec::CommandExecutor;
use command_parse::CommandParser;
use emacs_key_exec::EmacsKeyBindingExecutor;
use env_parse::EnvParser;
use input_rate_checker::InputRateChecker;
use input_validator::InputValidator;

pub use context::WorkflowContext;
pub use handler::WorkflowHandler;
use terminal_keycode::KeyCode;

pub fn autocomplete() -> Autocomplete {
    Autocomplete::default()
}

pub fn emacs_key(code: KeyCode) -> EmacsKeyBindingExecutor {
    EmacsKeyBindingExecutor::new(code)
}

pub fn env(name: String, value: String) -> EnvParser {
    let command_executor = CommandExecutor::default();
    let command_parser = CommandParser::new(command_executor);
    EnvParser::new(name, value, command_parser)
}

pub fn input_submit() -> InputRateChecker {
    let command_executor = CommandExecutor::default();
    let command_parser = CommandParser::new(command_executor);
    let input_validator = InputValidator::new(command_parser);
    InputRateChecker::new(input_validator)
}
