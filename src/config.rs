use std::time::Duration;

use crate::{
    priority::ForgetRules,
    ser::{parse_duration, parse_locking_strategy},
};
use py_spy::config::LockingStrategy;
use serde::Deserialize;

fn default_sampling_rate() -> u64 {
    10
}

fn default_window_width() -> Duration {
    Duration::from_secs(100)
}

fn default_subprocesses() -> bool {
    true
}
fn default_native() -> bool {
    true
}
fn default_dump_locals() -> u64 {
    1
}
fn default_rules() -> Vec<ForgetRules> {
    vec![ForgetRules::RectLinear {
        at_least: Duration::from_secs(60),
        ratio: 0.0,
    }]
}
fn default_update_period() -> Duration {
    Duration::from_millis(100)
}

fn default_locking_strategy() -> LockingStrategy {
    LockingStrategy::Lock
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "default_sampling_rate")]
    pub sampling_rate: u64,
    #[serde(deserialize_with = "parse_duration", default = "default_window_width")]
    pub window_width: Duration,
    #[serde(default = "default_subprocesses")]
    pub subprocesses: bool,
    #[serde(default = "default_native")]
    pub native: bool,
    // 1/128 max length of string repr of variable
    #[serde(default = "default_dump_locals")]
    pub dump_locals: u64,
    #[serde(default = "default_rules")]
    pub rules: Vec<ForgetRules>,
    #[serde(default = "default_update_period")]
    pub update_period: Duration,
    #[serde(
        deserialize_with = "parse_locking_strategy",
        default = "default_locking_strategy"
    )]
    pub locking_strategy: LockingStrategy,
}
