use crate::cli::Cli;
use crate::utils::verbose_println;
use clap::Parser;
use std::boxed::Box;
use std::error::Error;

mod cli;
mod model;
mod most_played;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    verbose_println(
        &args,
        format!(
            "API base: {}, API Limit: {:?}",
            args.api_url_base, args.request_limit
        ),
    );
    match args.command {
        cli::Commands::MostPlayed { limit, ref modes } => {
            most_played::most_played_command(&args, limit, modes).await?
        }
    }
    Ok(())
}
