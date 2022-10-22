use crate::cli::BeatmapMode;
use crate::model::{OutputBeatmap, OutputBeatmapSet, ReplayCount};
use clap::Parser;
use std::boxed::Box;
use std::collections::vec_deque::VecDeque;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display};
use std::hash::Hash;
use std::io::ErrorKind;
use std::process::exit;
use std::time::{Duration, Instant};
use serde::Deserialize;

mod cli;
mod model;

const MAX_REQUESTS_PER_MINUTE: usize = 50;

fn verbose_print<T: Display>(spec: &cli::Cli, message: T) {
    if spec.verbose {
        println!("{}", message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = cli::Cli::parse();
    match args.command {
        cli::Commands::MostPlayed { limit, ref modes } => {
            most_played_command(&args, limit, &modes).await?
        }
    }
    Ok(())
}

async fn most_played_command(
    spec: &cli::Cli,
    limit: Option<u16>,
    modes: &Vec<BeatmapMode>,
) -> Result<(), Box<dyn Error>> {
    verbose_print(spec, "Collecting most played beatmaps...");
    let most_played = get_most_played(spec, limit).await?;

    verbose_print(spec, "Data collected, processing data...");
    let summarized_data = summarize_most_played(most_played, limit, modes);

    if spec.output.is_some() {
        print_serializable(spec, summarized_data);
    } else {
        print_summary(spec, summarized_data);
    }
    Ok(())
}

fn print_serializable<T: serde::Serialize>(spec: &cli::Cli, data: Vec<T>) {
    if let Some(output) = spec.output {
        let output_string = match output {
            cli::OutputType::Json => serde_json::to_string_pretty(&data),
        };
        match output_string {
            Ok(json_str) => println!("{}", json_str),
            Err(e) => {
                eprintln!("Error when serializing: {}", e);
                exit(1);
            }
        }
    } else {
        eprintln!("Output type not specified");
        exit(1);
    }
}

fn get_mode_string(mode_data: &str) -> String {
    if mode_data == "osu" {
        "standard".to_string()
    } else if mode_data == "taiko" || mode_data == "mania" {
        mode_data.to_string()
    }
    else if mode_data == "fruits" {
        "catch".to_string()
    }
    else {
        "unknown mode".to_string()
    }
}

fn print_summary(_spec: &cli::Cli, data: Vec<OutputBeatmapSet>) {
    let mut total_plays = 0;
    let mut star_rating_freqs = Vec::new();
    let mut total_beatmapsets = 0;

    for set_summary in data {
        total_beatmapsets += 1;
        println!(
            "{} by {} (mapped by {})",
            set_summary.title, set_summary.artist, set_summary.creator
        );
        println!("\tPlays:   {}", set_summary.play_count);
        println!(
            "\tURL:     https://osu.ppy.sh/beatmapsets/{}\n",
            set_summary.set_id
        );
        println!("\tDifficulty Breakdown:");
        for bm in set_summary.beatmap_breakdown {
            println!(
                "\t\t {} stars, [{}]: {} plays",
                bm.difficulty_rating, get_mode_string(&bm.mode), bm.play_count
            );
            star_rating_freqs.push((bm.difficulty_rating, bm.play_count))
        }
        total_plays += set_summary.play_count;
    }

    println!("\nTotal plays: {}", total_plays);
    println!("Total beatmap sets: {}", total_beatmapsets);
    let weighed_avg = star_rating_freqs
        .iter()
        .fold(0.0, |acc, (stars, freq)| acc + *stars as f64 * *freq as f64)
        / total_plays as f64;
    println!("Average play star rating: {}", weighed_avg);
}

struct PrebuiltFilter<T: Hash + Eq> {
    values: HashSet<T>,
    is_values_allowlist: bool,
}

impl<T: Hash + Eq> PrebuiltFilter<T> {
    fn new(values: Vec<T>, allowlist: bool) -> Self {
        PrebuiltFilter {
            values: HashSet::from_iter(values.into_iter()),
            is_values_allowlist: allowlist,
        }
    }
    pub fn allow_values(values: Vec<T>) -> Self {
        Self::new(values, true)
    }
    // for blocklist
    // pub fn ban_values(values: Vec<T>) -> Self {
    //     Self::new(values, false)
    // }

    pub fn allows(&self, value: &T) -> bool {
        !(self.values.contains(value) ^ self.is_values_allowlist)
    }
}

fn modes_to_api_values(modes: &Vec<BeatmapMode>) -> Vec<String> {
    if modes.is_empty() {
        vec![
            "osu".to_string(),
            "mania".to_string(),
            "taiko".to_string(),
            "fruits".to_string(),
        ]
    } else {
        modes
            .iter()
            .map(|mode| match mode {
                BeatmapMode::Standard => "osu".to_string(),
                BeatmapMode::Mania => "mania".to_string(),
                BeatmapMode::Taiko => "taiko".to_string(),
                BeatmapMode::Catch => "fruits".to_string(),
            })
            .collect()
    }
}

fn summarize_most_played(
    data: Vec<ReplayCount>,
    limit: Option<u16>,
    modes: &Vec<BeatmapMode>,
) -> Vec<OutputBeatmapSet> {
    const REPLAY_HASHABLE_DIFFICULTY: fn(&ReplayCount) -> u32 =
        |replay: &ReplayCount| (replay.beatmap.difficulty_rating * 10000.0) as u32;
    let mut set_id_map: HashMap<u32, OutputBeatmapSet> = HashMap::new();
    let mut set_diffs_processed: HashMap<u32, HashSet<u32>> = HashMap::new();
    let mode_filter = PrebuiltFilter::allow_values(modes_to_api_values(modes));

    for replay_info in data {
        if !mode_filter.allows(&replay_info.beatmap.mode) {
            continue;
        }

        if let Some(info) = set_id_map.get_mut(&replay_info.beatmapset.id) {
            // unwrap is safe because set_diffs_processed[id] gets occupied in the else branch along
            // with set_id_map[id] which has just been checked
            let difficulty_map = set_diffs_processed
                .get_mut(&replay_info.beatmapset.id)
                .unwrap();

            if difficulty_map.insert(REPLAY_HASHABLE_DIFFICULTY(&replay_info)) {
                info.play_count += replay_info.count as u64;
                info.beatmap_breakdown
                    .push(extract_beatmap_output(&replay_info));
            }
        } else {
            set_id_map.insert(
                replay_info.beatmapset.id,
                extract_beatmap_set_output(&replay_info),
            );
            set_diffs_processed.insert(
                replay_info.beatmapset.id,
                HashSet::from([REPLAY_HASHABLE_DIFFICULTY(&replay_info)]),
            );
        }
    }

    let mut result: Vec<OutputBeatmapSet> = set_id_map.into_iter().map(|(_, val)| val).collect();
    result.sort_by(|x, y| y.play_count.cmp(&x.play_count));

    if let Some(l) = limit {
        let mut limited_result = Vec::new();
        for set in result {
            if limited_result.len() < l.into() {
                limited_result.push(set);
            } else {
                break;
            }
        }

        return limited_result;
    }

    result
}

fn extract_beatmap_output(replay: &ReplayCount) -> OutputBeatmap {
    OutputBeatmap {
        status: replay.beatmap.status.clone(),
        difficulty_rating: replay.beatmap.difficulty_rating,
        id: replay.beatmap_id,
        mode: replay.beatmap.mode.clone(),
        play_count: replay.count,
    }
}

fn extract_beatmap_set_output(replay: &ReplayCount) -> OutputBeatmapSet {
    OutputBeatmapSet {
        set_id: replay.beatmapset.id,
        artist: replay.beatmapset.artist.clone(),
        creator: replay.beatmapset.creator.clone(),
        title: replay.beatmapset.title.clone(),
        play_count: replay.count as u64,
        beatmap_breakdown: vec![extract_beatmap_output(replay)],
    }
}

async fn wait_on_limiter(spec: &cli::Cli, limiter: &OsuAPILimiter) {
    let wait_time = limiter.time_until_next_request();
    if wait_time.is_zero() {
        return;
    }
    verbose_print(
        spec,
        format!(
            "Sleeping on API limit ({}) for {} seconds",
            MAX_REQUESTS_PER_MINUTE,
            wait_time.as_secs()
        ),
    );
    tokio::time::sleep(wait_time).await;
}

async fn exponential_wait_request_attempt<D: for<'a> Deserialize<'a>>(spec: &cli::Cli, url: &str, max_retries: u32) -> Result<D, Box<dyn Error>>
{
    let mut retry_count = 0;
    let mut last_error: Box<dyn Error> = Box::new(std::io::Error::new(ErrorKind::Unsupported, "This error should never happen!"));
    let mut limiter = OsuAPILimiter::new(MAX_REQUESTS_PER_MINUTE);

    while retry_count <= max_retries {
        wait_on_limiter(spec, &limiter).await;
        let raw_response = reqwest::get(url).await;
        limiter.register_request_performed();
        let json_result = match raw_response {
            Ok(r) => r.json::<D>().await,
            Err(e) => {
                verbose_print(spec, format!("Error when performing a request!\nError: {}\nRetry counter: {}\\{}", e.to_string(), retry_count, max_retries));
                last_error = Box::new(e);
                tokio::time::sleep(Duration::new(i32::from(2).pow(retry_count) as u64, 0)).await;
                retry_count += 1;
                continue;
            }
        };

        match json_result {
            Ok(json) => return Ok(json),
            Err(e) => {
                verbose_print(spec, format!("Error when performing a request!\nError: {}\nRetry counter: {}\\{}", e.to_string(), retry_count, max_retries));
                last_error = Box::new(e);
                tokio::time::sleep(Duration::new(i32::from(2).pow(retry_count)  as u64, 0)).await;
                retry_count += 1;
                continue;
            }
        };
    };
    return Err(last_error);
}

async fn get_most_played(
    spec: &cli::Cli,
    limit: Option<u16>,
) -> Result<Vec<ReplayCount>, Box<dyn Error>> {
    let mut replay_counts = Vec::new();
    let mut offset = 0;
    const PAGE_SIZE: u32 = 100;

    loop {

        let url = format!(
            "{}/users/{}/beatmapsets/most_played?limit={}&offset={}",
            spec.api_url_base,
            spec.user_id,
            PAGE_SIZE,
            offset
        );
        verbose_print(spec, format!("URL: {}", url));

        let mut resp: Vec<ReplayCount> = exponential_wait_request_attempt(spec, &url, 3).await?;

        if resp.is_empty() {
            break;
        }

        replay_counts.append(&mut resp);

        if limit.is_some() && replay_counts.len() >= usize::from(limit.unwrap()) {
            break;
        }

        offset += 1;
    }

    Ok(replay_counts)
}

struct OsuAPILimiter {
    limit: usize,
    starts: VecDeque<Instant>,
}

impl OsuAPILimiter {
    pub fn new(limit: usize) -> Self {
        OsuAPILimiter {
            limit,
            starts: VecDeque::new(),
        }
    }

    pub fn time_until_next_request(&self) -> Duration {
        if self.starts.len() < self.limit {
            return Duration::new(0, 0);
        }
        const MICROS_MINUTE: u128 = 60 * 1000 * 1000;
        if self
            .starts
            .front()
            .map_or(true, |x| x.elapsed().as_micros() > MICROS_MINUTE)
        {
            return Duration::new(0, 0);
        }

        let furthest_instant = self.starts.front().unwrap();
        let micros_until_request_allowed = MICROS_MINUTE - furthest_instant.elapsed().as_micros();
        let seconds_until_request_allowed = (micros_until_request_allowed / 1000 / 1000) + 1;

        let u64_seconds_until_request_allowed = if seconds_until_request_allowed > 60 {
            60
        } else {
            seconds_until_request_allowed as u64
        };
        Duration::new(u64_seconds_until_request_allowed, 0)
    }

    pub fn register_request_performed(&mut self) {
        if self.starts.len() >= self.limit {
            self.starts.pop_front();
        }

        self.starts.push_back(Instant::now());
    }
}
