pub mod fs;
pub mod ssh;

mod set;

pub use set::TimedHashSet;

pub const NEWLINE: &'static str = "\n\r";
