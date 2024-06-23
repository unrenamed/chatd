pub mod fs;
pub mod ssh;

mod set;
mod unicode;

pub use set::TimedHashSet;
pub use unicode::display_width;

pub const NEWLINE: &'static str = "\n\r";
