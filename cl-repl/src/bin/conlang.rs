use cl_repl::{cli::run, tools::is_terminal};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if is_terminal() {
        println!(
            "--- {} v{} 💪🦈 ---",
            env!("CARGO_BIN_NAME"),
            env!("CARGO_PKG_VERSION"),
        );
    }
    run(argh::from_env())
}
