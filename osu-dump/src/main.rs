use crate::model::{OutputBeatmap, OutputBeatmapSet, ReplayCount};
use clap::Parser;
use std::boxed::Box;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::time::{Duration, Instant};

mod cli;
mod model;

const MAX_REQUESTS_PER_MINUTE: usize = 50;

fn verbose_print<T: Display>(spec: &cli::Cli, message: T) {
    if spec.verbose {
        println!("{}", message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::parse();
    match args.command {
        cli::Commands::MostPlayed { limit } => most_played_command(&args, limit).await?,
    }
    Ok(())
}

async fn most_played_command(spec: &cli::Cli, limit: Option<u16>) -> Result<(), Box<dyn Error>> {
    verbose_print(spec, "Collecting most played beatmaps...");
    let most_played = get_most_played(spec, limit).await?;

    verbose_print(spec, "Data collected, processing data...");
    let summarized_data = summarize_most_played(most_played, limit);

    if spec.output.is_some() {
        // TODO
        print_serializable(spec, summarized_data);
    } else {
        print_summary(spec, summarized_data);
    }
    Ok(())
}

fn print_serializable<T: serde::Serialize>(spec: &cli::Cli, _data: Vec<T>) {
    // TODO
    if let Some(output) = spec.output {
        match output {
            cli::OutputType::Json => println!("JSON"),
        }
    } else {
        println!("ERROR");
    }
}

fn print_summary(_spec: &cli::Cli, data: Vec<OutputBeatmapSet>) {
    let mut total_plays = 0;
    let mut star_rating_freqs = Vec::new();

    for set_summary in data {
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
                "\t\t {} stars: {} plays",
                bm.difficulty_rating, bm.play_count
            );
            star_rating_freqs.push((bm.difficulty_rating, bm.play_count))
        }
        total_plays += set_summary.play_count;
    }

    println!("\nTotal plays: {}", total_plays);
    let weighed_avg = star_rating_freqs
        .iter()
        .fold(0.0, |acc, (stars, freq)| acc + *stars as f64 * *freq as f64)
        / total_plays as f64;
    println!("Average play star rating: {}", weighed_avg);
}

fn summarize_most_played(
    data: Vec<model::ReplayCount>,
    limit: Option<u16>,
) -> Vec<OutputBeatmapSet> {
    let mut set_id_map: HashMap<u32, OutputBeatmapSet> = HashMap::new();

    for replay_info in data {
        if let Some(info) = set_id_map.get_mut(&replay_info.beatmapset.id) {
            // TODO efficiency
            if !info
                .beatmap_breakdown
                .iter()
                .any(|x| x.id == replay_info.beatmap_id)
            {
                // TODO parametrize
                if replay_info.beatmap.mode.to_lowercase() != "osu" {
                    continue;
                }

                info.play_count += replay_info.count as u64;
                info.beatmap_breakdown
                    .push(extract_beatmap_output(&replay_info));
            }
        } else {
            set_id_map.insert(
                replay_info.beatmapset.id,
                extract_beatmap_set_output(&replay_info),
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

fn extract_beatmap_output(replay: &model::ReplayCount) -> OutputBeatmap {
    OutputBeatmap {
        status: replay.beatmap.status.clone(),
        difficulty_rating: replay.beatmap.difficulty_rating,
        id: replay.beatmap_id,
        mode: replay.beatmap.mode.clone(),
        play_count: replay.count,
    }
}

fn extract_beatmap_set_output(replay: &model::ReplayCount) -> OutputBeatmapSet {
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

async fn get_most_played(
    spec: &cli::Cli,
    limit: Option<u16>,
) -> Result<Vec<model::ReplayCount>, Box<dyn Error>> {
    let mut replay_counts = Vec::new();
    let mut offset = 0;
    const PAGE_SIZE: u32 = 100;
    let mut limiter = OsuAPILimiter::new(MAX_REQUESTS_PER_MINUTE);

    loop {
        wait_on_limiter(spec, &limiter).await;
        let url = format!(
            "https://osu.ppy.sh/users/{}/beatmapsets/most_played?limit={}&offset={}",
            spec.user_id.as_str(),
            PAGE_SIZE,
            offset
        );
        verbose_print(spec, format!("URL: {}", url));
        limiter.register_request_performed();

        let mut resp = reqwest::get(url).await?.json::<Vec<ReplayCount>>().await?;

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
