use cl_repl::{args::Args, cli::CLI};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    CLI::from(Args::new().parse().unwrap_or_default()).run()
}
