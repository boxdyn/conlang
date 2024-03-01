use cl_repl::cli::run;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    run(argh::from_env())
}
