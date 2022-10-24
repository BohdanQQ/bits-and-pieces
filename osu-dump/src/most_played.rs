use crate::cli::{BeatmapMode, Cli};
use crate::model::{BeatmapPlaycount, OutputBeatmap, OutputBeatmapSet};
use crate::utils::{
    all_or_modes_to_api_values, exponential_wait_request_attempt, print_serializable,
    verbose_println, OsuAPILimiter, PrebuiltFilter,
};
use std::boxed::Box;
use std::collections::{HashMap, HashSet};
use std::error::Error;

pub async fn most_played_command(
    spec: &Cli,
    limit: Option<u64>,
    modes: &Vec<BeatmapMode>,
) -> Result<(), Box<dyn Error>> {
    verbose_println(spec, "Collecting most played beatmaps...");
    let most_played = get_most_played_raw(spec, limit).await?;

    verbose_println(spec, "Data collected, processing data...");
    let summarized_data = summarize_most_played(most_played, limit, modes);

    if let Some(out_type) = spec.output {
        print_serializable(out_type, summarized_data);
    } else {
        print_summary(summarized_data);
    }
    Ok(())
}

/// # Returns
/// user-friendly identifier translated from a raw mode identifier, "unknown mode" on unknown mode identifier
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

/// Prints data summary on stdout in human-readable format
fn print_summary(data: Vec<OutputBeatmapSet>) {
    let mut total_plays = 0;
    let mut star_rating_freqs = Vec::new();
    let mut total_beatmapsets = 0;

    for set_summary in data {
        total_beatmapsets += 1;
        println!(
            "{}\nby {}\nmapped by {}",
            set_summary.title, set_summary.artist, set_summary.creator
        );
        println!("\tPlays:   {}", set_summary.play_count);
        println!(
            "\tURL:     https://osu.ppy.sh/beatmapsets/{}\n",
            set_summary.set_id
        );
        println!("\tDifficulty Breakdown:");
        for beatmap in set_summary.beatmap_breakdown {
            println!(
                "\t\t {:.2} stars, [{}]: {:5} plays",
                beatmap.difficulty_rating,
                get_mode_string(&beatmap.mode),
                beatmap.play_count
            );
            star_rating_freqs.push((beatmap.difficulty_rating, beatmap.play_count))
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

fn summarize_most_played(
    data: Vec<BeatmapPlaycount>,
    limit: Option<u64>,
    modes: &Vec<BeatmapMode>,
) -> Vec<OutputBeatmapSet> {
    let mode_filter = PrebuiltFilter::allow_values(all_or_modes_to_api_values(modes));

    let mut result =
        aggregate_difficulties_with_filter(data, move |x| mode_filter.allows(&x.beatmap.mode));
    result.sort_by(|x, y| y.play_count.cmp(&x.play_count));

    try_limit_output(result, limit)
}

fn aggregate_difficulties_with_filter<F>(
    data: Vec<BeatmapPlaycount>,
    filter: F,
) -> Vec<OutputBeatmapSet>
where
    F: Fn(&BeatmapPlaycount) -> bool,
{
    const REPLAY_HASHABLE_DIFFICULTY: fn(&BeatmapPlaycount) -> u32 =
        |replay: &BeatmapPlaycount| (replay.beatmap.difficulty_rating * 10000.0) as u32;
    let mut set_id_map: HashMap<u32, OutputBeatmapSet> = HashMap::new();
    let mut set_diffs_processed: HashMap<u32, HashSet<u32>> = HashMap::new();

    for replay_info in data {
        if !filter(&replay_info) {
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
                    .push(OutputBeatmap::from(&replay_info));
            }
        } else {
            set_id_map.insert(
                replay_info.beatmapset.id,
                OutputBeatmapSet::from(&replay_info),
            );
            set_diffs_processed.insert(
                replay_info.beatmapset.id,
                HashSet::from([REPLAY_HASHABLE_DIFFICULTY(&replay_info)]),
            );
        }
    }
    set_id_map.into_iter().map(|(_, val)| val).collect()
}

fn try_limit_output(target: Vec<OutputBeatmapSet>, limit: Option<u64>) -> Vec<OutputBeatmapSet> {
    if let Some(l) = limit {
        let mut limited_result = Vec::new();
        for set in target {
            if u64::try_from(limited_result.len()).unwrap() < l {
                limited_result.push(set);
            } else {
                break;
            }
        }

        return limited_result;
    }

    target
}

async fn get_most_played_raw(
    spec: &Cli,
    limit: Option<u64>,
) -> Result<Vec<BeatmapPlaycount>, Box<dyn Error>> {
    let mut replay_counts = Vec::new();
    let mut unique_set_ids = HashSet::new();
    let mut offset = 0;
    const PAGE_SIZE: u32 = 100;

    let mut limiter = OsuAPILimiter::new(
        spec.request_limit.requests,
        spec.request_limit.timeframe_secs,
    );
    loop {
        let url = format!(
            "{}/users/{}/beatmapsets/most_played?limit={}&offset={}",
            spec.api_url_base, spec.user_id, PAGE_SIZE, offset
        );

        let mut resp: Vec<BeatmapPlaycount> =
            exponential_wait_request_attempt(spec, &url, 3, &mut limiter).await?;

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
