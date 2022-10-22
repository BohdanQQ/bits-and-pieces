use crate::cli::{BeatmapMode, Cli};
use crate::model::{OutputBeatmap, OutputBeatmapSet, ReplayCount};
use crate::utils::verbose_println;
use clap::Parser;
use std::boxed::Box;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::Hash;
use std::process::exit;

mod cli;
mod model;
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
            most_played_command(&args, limit, modes).await?
        }
    }
    Ok(())
}

async fn most_played_command(
    spec: &Cli,
    limit: Option<u64>,
    modes: &Vec<BeatmapMode>,
) -> Result<(), Box<dyn Error>> {
    verbose_println(spec, "Collecting most played beatmaps...");
    let most_played = get_most_played(spec, limit).await?;

    verbose_println(spec, "Data collected, processing data...");
    let summarized_data = summarize_most_played(most_played, limit, modes);

    if spec.output.is_some() {
        print_serializable(spec, summarized_data);
    } else {
        print_summary(spec, summarized_data);
    }
    Ok(())
}

fn print_serializable<T: serde::Serialize>(spec: &Cli, data: Vec<T>) {
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
    } else if mode_data == "fruits" {
        "catch".to_string()
    } else {
        "unknown mode".to_string()
    }
}

fn print_summary(_spec: &Cli, data: Vec<OutputBeatmapSet>) {
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
                bm.difficulty_rating,
                get_mode_string(&bm.mode),
                bm.play_count
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

fn summarize_most_played(
    data: Vec<ReplayCount>,
    limit: Option<u64>,
    modes: &Vec<BeatmapMode>,
) -> Vec<OutputBeatmapSet> {
    const REPLAY_HASHABLE_DIFFICULTY: fn(&ReplayCount) -> u32 =
        |replay: &ReplayCount| (replay.beatmap.difficulty_rating * 10000.0) as u32;
    let mut set_id_map: HashMap<u32, OutputBeatmapSet> = HashMap::new();
    let mut set_diffs_processed: HashMap<u32, HashSet<u32>> = HashMap::new();
    let mode_filter = PrebuiltFilter::allow_values(utils::all_or_modes_to_api_values(modes));

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
            if u64::try_from(limited_result.len()).unwrap() < l {
                limited_result.push(set);
            } else {
                break;
            }
        }

        return limited_result;
    }

    result
}

async fn get_most_played(
    spec: &Cli,
    limit: Option<u64>,
) -> Result<Vec<ReplayCount>, Box<dyn Error>> {
    let mut replay_counts = Vec::new();
    let mut unique_set_ids = HashSet::new();
    let mut offset = 0;
    const PAGE_SIZE: u32 = 100;

    let mut limiter = utils::OsuAPILimiter::new(
        spec.request_limit.requests,
        spec.request_limit.timeframe_secs,
    );
    loop {
        let url = format!(
            "{}/users/{}/beatmapsets/most_played?limit={}&offset={}",
            spec.api_url_base, spec.user_id, PAGE_SIZE, offset
        );

        let mut resp: Vec<ReplayCount> =
            utils::exponential_wait_request_attempt(spec, &url, 3, &mut limiter).await?;

        if resp.is_empty() {
            break;
        }

        for count in &resp {
            unique_set_ids.insert(count.beatmapset.id);
        }

        replay_counts.append(&mut resp);

        if limit.is_some() && u64::try_from(unique_set_ids.len()).unwrap() >= limit.unwrap() {
            break;
        }

        offset += 1;
    }

    Ok(replay_counts)
}
