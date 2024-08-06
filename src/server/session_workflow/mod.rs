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

use std::io::Write;
use terminal_keycode::KeyCode;

use crate::terminal::CloseHandle;

#[cfg(not(tarpaulin_include))]
pub fn autocomplete<H: Clone + Write + CloseHandle + Send>() -> Autocomplete<H> {
    Autocomplete::new()
}

#[cfg(not(tarpaulin_include))]
pub fn emacs_key<H: Clone + Write + CloseHandle + Send>(
    code: KeyCode,
) -> EmacsKeyBindingExecutor<H> {
    EmacsKeyBindingExecutor::new(code)
}

#[cfg(not(tarpaulin_include))]
pub fn env<H: Clone + Write + CloseHandle + Send + 'static>(
    name: String,
    value: String,
) -> EnvParser<H> {
    let command_executor = CommandExecutor::new();
    let command_parser = CommandParser::new(command_executor);
    EnvParser::new(name, value, command_parser)
}

#[cfg(not(tarpaulin_include))]
pub fn input_submit<H: Clone + Write + CloseHandle + Send + 'static>() -> InputRateChecker<H> {
    let command_executor = CommandExecutor::new();
    let command_parser = CommandParser::new(command_executor);
    let input_validator = InputValidator::new(command_parser);
    InputRateChecker::new(input_validator)
}
