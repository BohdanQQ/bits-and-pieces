use crate::cli::{BeatmapMode, Cli};
use serde::Deserialize;
use std::boxed::Box;
use std::collections::vec_deque::VecDeque;
use std::error::Error;
use std::fmt::Display;
use std::io::ErrorKind;
use std::time::{Duration, Instant};

#[path = "./cli.rs"]
mod cli;
#[path = "./model.rs"]
mod model;

const MAX_REQUESTS_PER_MINUTE: usize = 50;

pub fn verbose_print<T: Display>(spec: &Cli, message: T) {
    if spec.verbose {
        println!("{}", message);
    }
}

async fn wait_on_limiter(spec: &Cli, limiter: &OsuAPILimiter) {
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

pub async fn exponential_wait_request_attempt<D: for<'a> Deserialize<'a>>(
    spec: &Cli,
    url: &str,
    max_retries: u32,
) -> Result<D, Box<dyn Error>> {
    let mut retry_count = 0;
    let mut last_error: Box<dyn Error> = Box::new(std::io::Error::new(
        ErrorKind::Unsupported,
        "This error should never propagate!",
    ));
    let mut limiter = OsuAPILimiter::new(MAX_REQUESTS_PER_MINUTE);

    while retry_count <= max_retries {
        wait_on_limiter(spec, &limiter).await;
        let raw_response = reqwest::get(url).await;
        limiter.register_request_performed();
        let json_result = match raw_response {
            Ok(r) => r.json::<D>().await,
            Err(e) => {
                verbose_print(
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
                verbose_print(
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
