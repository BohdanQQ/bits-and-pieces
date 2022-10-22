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

    /// API URL to use. It is recommended to setup a local caching proxy to save time and bandwidth.
    #[arg(short, long, default_value("https://osu.ppy.sh"))]
    pub api_url_base: String
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Lists Most Played Beatmaps. Data collection may take very long time depending on the amount of beatmaps played.
    MostPlayed {
        /// Limits the amount of beatmaps outputted. LIMITS ACCURACY because not all data will be fetched from the Osu! API.
        #[arg(short, long)]
        limit: Option<u16>,

        #[clap(help("Osu beatmap modes to filter. Accepts all by default."))]
        modes: Vec<BeatmapMode>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputType {
    Json,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum BeatmapMode {
    Standard,
    Mania,
    Taiko,
    Catch,
}
