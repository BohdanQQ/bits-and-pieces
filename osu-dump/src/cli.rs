use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(author = "BohdanQQ", version = "0.0.2", about = "An Osu! profile data extraction tool.", long_about = None)]
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
    #[arg(short('u'), long, default_value("https://osu.ppy.sh"))]
    pub api_url_base: String,

    /// API request limit in the format "N:M" where N is the maximum number of calls in M seconds. Use 0 to represent "no limits".
    #[arg(short('r'), long, value_parser = api_limit_parser, default_value("50:60"))]
    pub request_limit: RequestLimit,
}

#[derive(Clone, Debug)]
pub struct RequestLimit {
    pub requests: u16,
    pub timeframe_secs: u16,
}

fn api_limit_parser(s: &str) -> Result<RequestLimit, String> {
    let split: Vec<&str> = s.split(':').collect();

    if split.len() != 2 {
        return Err("Invalid API Limit format, use the format \"N:M\" where N is the maximum number of calls in M seconds. Use 0 to represent \"no limits\".".to_string());
    }

    let requests = split[0]
        .parse()
        .map_err(|_| format!("\"{}\" is not a valid number", split[0]))?;
    let timeframe_secs = split[1]
        .parse()
        .map_err(|_| format!("\"{}\" is not a valid number", split[1]))?;

    Ok(RequestLimit {
        requests,
        timeframe_secs,
    })
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
