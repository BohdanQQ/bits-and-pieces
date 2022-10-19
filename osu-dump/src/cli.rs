use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(author = "BohdanQQ", version = "0.0.1", about = "An Osu! profile data extraction tool.", long_about = None)]
pub struct Cli {
    /// User ID
    pub user_id: String,

    /// Request verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,

    /// Output type
    #[arg(short, long)]
    pub output: Option<OutputType>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Lists Most Played Beatmaps
    MostPlayed {
        /// Limits the amount of beatmaps outputted
        #[arg(short, long)]
        limit: Option<u16>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputType {
    Json,
}
