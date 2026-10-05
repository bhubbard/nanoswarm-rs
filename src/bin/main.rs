use clap::Parser;
use nanoswarm::cli::{run_cli, CliArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    let exit_code = run_cli(args).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
