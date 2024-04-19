use cl_repl::cli::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(argh::from_env())
}
