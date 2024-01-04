use cl_frontend::{args::Args, cli::CLI};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // parse args
    let args = Args::new().parse().unwrap_or_default();
    let mut cli = CLI::from(args);
    cli.run()
}
