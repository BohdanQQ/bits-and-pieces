use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplayCount {
    pub beatmap_id: u32,
    pub count: u32,
    pub beatmap: Beatmap,
    pub beatmapset: BeatmapSet,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Beatmap {
    pub difficulty_rating: f32,
    pub id: u32,
    pub mode: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BeatmapSet {
    pub id: u32,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub title: String,
    pub title_unicode: String,
}

#[derive(Serialize)]
pub struct OutputBeatmapSet {
    pub set_id: u32,
    pub artist: String,
    pub creator: String,
    pub title: String,
    pub play_count: u64,
    pub beatmap_breakdown: Vec<OutputBeatmap>,
}

impl OutputBeatmapSet {
    pub fn from(replay: &ReplayCount) -> Self {
        OutputBeatmapSet {
            set_id: replay.beatmapset.id,
            artist: replay.beatmapset.artist.clone(),
            creator: replay.beatmapset.creator.clone(),
            title: replay.beatmapset.title.clone(),
            play_count: replay.count as u64,
            beatmap_breakdown: vec![OutputBeatmap::from(replay)],
        }
    }
}

impl OutputBeatmap {
    pub fn from(replay: &ReplayCount) -> Self {
        OutputBeatmap {
            status: replay.beatmap.status.clone(),
            difficulty_rating: replay.beatmap.difficulty_rating,
            id: replay.beatmap_id,
            mode: replay.beatmap.mode.clone(),
            play_count: replay.count,
        }
    }
}

#[derive(Serialize)]
pub struct OutputBeatmap {
    pub id: u32,
    pub difficulty_rating: f32,
    pub mode: String,
    pub status: String,
    pub play_count: u32,
}
