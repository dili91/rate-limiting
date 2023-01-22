//! Module that includes builders to construct instances of the 2 rate limiter types. Used internally.
pub mod sliding_window;
pub mod token_bucket;

const DEFAULT_REDIS_HOST: &str = "127.0.0.1";
const DEFAULT_REDIS_PORT: u16 = 6379;

#[derive(Clone)]
/// Represent the Redis configuration object
pub struct RedisSettings {
    /// The host of the Redis server used.
    pub host: String,
    /// The port of the Redis server used.
    pub port: u16,
}
