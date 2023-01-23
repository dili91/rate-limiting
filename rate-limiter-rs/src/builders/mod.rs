//! Module that includes builders to construct instances of the 2 rate limiter types. Used internally.

use std::time::Duration;
pub mod fixed_window;
pub mod sliding_window;

const DEFAULT_REDIS_HOST: &str = "127.0.0.1";
const DEFAULT_REDIS_PORT: u16 = 6379;
const DEFAULT_WINDOW_SIZE: u64 = 5;
const DEFAULT_WINDOW_DURATION: Duration = Duration::from_secs(15);

#[derive(Clone)]
/// Represent the Redis configuration object
pub struct RedisSettings {
    /// The host of the Redis server used.
    pub host: String,
    /// The port of the Redis server used.
    pub port: u16,
}
