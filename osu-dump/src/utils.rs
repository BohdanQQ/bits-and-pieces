use crate::cli::{BeatmapMode, Cli, OutputType};
use serde::Deserialize;
use std::boxed::Box;
use std::collections::vec_deque::VecDeque;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::hash::Hash;
use std::io::ErrorKind;
use std::process::exit;
use std::time::{Duration, Instant};

pub fn verbose_print<T: Display>(spec: &Cli, message: T) {
    if spec.verbose {
        print!("{}", message);
    }
}

pub fn verbose_println<T: Display>(spec: &Cli, message: T) {
    if spec.verbose {
        println!("{}", message);
    }
}

async fn wait_on_limiter(spec: &Cli, limiter: &OsuAPILimiter) {
    let wait_time = limiter.time_until_next_request();
    if wait_time.is_zero() {
        return;
    }
    verbose_println(
        spec,
        format!(
            "Sleeping on API limit ({}) for {} seconds",
            spec.request_limit.requests,
            wait_time.as_secs()
        ),
    );
    tokio::time::sleep(wait_time).await;
}

pub async fn exponential_wait_request_attempt<D: for<'a> Deserialize<'a>>(
    spec: &Cli,
    url: &str,
    max_retries: u32,
    limiter: &mut OsuAPILimiter,
) -> Result<D, Box<dyn Error>> {
    let mut retry_count = 0;
    let mut last_error: Box<dyn Error> = Box::new(std::io::Error::new(
        ErrorKind::Unsupported,
        "This error should never propagate!",
    ));

    while retry_count <= max_retries {
        wait_on_limiter(spec, limiter).await;

        verbose_print(spec, format!("GET {}", url));
        let raw_response = reqwest::get(url).await;
        verbose_println(spec, " finished");

        limiter.register_request_performed();
        let json_result = match raw_response {
            Ok(r) => r.json::<D>().await,
            Err(e) => {
                verbose_println(
                    spec,
                    format!(
                        "Error when performing a request!\nError: {}\nRetry counter: {}\\{}",
                        e, retry_count, max_retries
                    ),
                );
                last_error = Box::new(e);
                tokio::time::sleep(Duration::new(2_i32.pow(retry_count) as u64, 0)).await;
                retry_count += 1;
                continue;
            }
        };

        match json_result {
            Ok(json) => return Ok(json),
            Err(e) => {
                verbose_println(
                    spec,
                    format!(
                        "Error when performing a request!\nError: {}\nRetry counter: {}\\{}",
                        e, retry_count, max_retries
                    ),
                );
                last_error = Box::new(e);
                tokio::time::sleep(Duration::new(2_i32.pow(retry_count) as u64, 0)).await;
                retry_count += 1;
                continue;
            }
        };
    }
    Err(last_error)
}

pub struct OsuAPILimiter {
    limit: u16,
    timeframe_secs: u16,
    starts: VecDeque<Instant>,
}

impl OsuAPILimiter {
    pub fn new(limit: u16, timeframe_secs: u16) -> Self {
        OsuAPILimiter {
            limit,
            timeframe_secs,
            starts: VecDeque::new(),
        }
    }

    pub fn time_until_next_request(&self) -> Duration {
        if self.limit == 0 || self.starts.len() < usize::from(self.limit) {
            return Duration::new(0, 0);
        }
        let time_frame_micros: u128 = u128::from(self.timeframe_secs) * 1000 * 1000;
        if self
            .starts
            .front()
            .map_or(true, |x| x.elapsed().as_micros() >= time_frame_micros)
        {
            return Duration::new(0, 0);
        }

        let furthest_instant = self.starts.front().unwrap();
        let micros_until_request_allowed =
            time_frame_micros - furthest_instant.elapsed().as_micros();
        let seconds_until_request_allowed = (micros_until_request_allowed / 1000 / 1000) + 1;

        let u64_seconds_until_request_allowed =
            if seconds_until_request_allowed > u128::from(self.timeframe_secs) {
                u64::from(self.timeframe_secs)
            } else {
                seconds_until_request_allowed as u64
            };
        Duration::new(u64_seconds_until_request_allowed, 0)
    }

    pub fn register_request_performed(&mut self) {
        if self.limit == 0 {
            return;
        }

        if self.starts.len() >= usize::from(self.limit) {
            self.starts.pop_front();
        }

        self.starts.push_back(Instant::now());
    }
}
pub fn mode_to_api_value(mode: &BeatmapMode) -> String {
    match mode {
        BeatmapMode::Standard => "osu".to_string(),
        BeatmapMode::Mania => "mania".to_string(),
        BeatmapMode::Taiko => "taiko".to_string(),
        BeatmapMode::Catch => "fruits".to_string(),
    }
}

pub fn all_or_modes_to_api_values(modes: &Vec<BeatmapMode>) -> Vec<String> {
    if modes.is_empty() {
        vec![
            "osu".to_string(),
            "mania".to_string(),
            "taiko".to_string(),
            "fruits".to_string(),
        ]
    } else {
        modes.iter().map(mode_to_api_value).collect()
    }
}

pub fn print_serializable<T: serde::Serialize>(output_type: OutputType, data: T) {
    let output_string = match output_type {
        OutputType::Json => serde_json::to_string_pretty(&data),
    };
    match output_string {
        Ok(json_str) => println!("{}", json_str),
        Err(e) => {
            eprintln!("Error when serializing: {}", e);
            exit(1);
        }
    }
}

pub struct PrebuiltFilter<T: Hash + Eq> {
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
