use cl_repl::{args, cli::run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(args::Args::args()?)
}
