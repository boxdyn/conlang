//! ANSI escape sequences

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m"; // the color of type checker mode
pub const CYAN: &str = "\x1b[36m";
pub const BRIGHT_GREEN: &str = "\x1b[92m";
pub const BRIGHT_BLUE: &str = "\x1b[94m";
pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
pub const BRIGHT_CYAN: &str = "\x1b[96m";
pub const RESET: &str = "\x1b[0m";
pub const OUTPUT: &str = "\x1b[38;5;117m";

pub const CLEAR_LINES: &str = "\x1b[G\x1b[J";
pub const CLEAR_ALL: &str = "\x1b[H\x1b[2J";
