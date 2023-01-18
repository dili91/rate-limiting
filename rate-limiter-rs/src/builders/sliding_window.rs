use std::time::Duration;

use redis::Client as RedisClient;

use crate::{errors::RateLimiterError, rate_limiters::sliding_window::SlidingWindowRateLimiter};

use super::{RedisSettings, DEFAULT_REDIS_HOST, DEFAULT_REDIS_PORT};

const DEFAULT_WINDOW_SIZE: u64 = 5;
const DEFAULT_WINDOW_DURATION: Duration = Duration::from_secs(15);

#[derive(Default)]
pub struct SlidingWindowRateLimiterBuilder {
    window_size: Option<u64>,
    window_duration: Option<Duration>,
    /// The configuration of the underlying [Redis](https://redis.io) server used
    redis_settings: Option<RedisSettings>,
}

impl SlidingWindowRateLimiterBuilder {
    /// Setter for the rate limiter bucket size.
    pub fn with_window_size(mut self, size: u64) -> Self {
        self.window_size = Some(size);
        self
    }

    /// Setter for the rate limiter bucket validity.
    pub fn with_window_duration(mut self, window_duration: Duration) -> Self {
        self.window_duration = Some(window_duration);
        self
    }

    /// Setter for the underlying Redis server settings.
    pub fn with_redis_settings(mut self, redis_settings: RedisSettings) -> Self {
        self.redis_settings = Some(redis_settings);
        self
    }

    /// Function that tries to build the rate limiter.
    pub fn build(&self) -> Result<SlidingWindowRateLimiter, RateLimiterError> {
        let redis_client = self
            .redis_settings
            .as_ref()
            .map(|rs| RedisClient::open(format!("redis://{0}:{1}", rs.host, rs.port)))
            .unwrap_or_else(|| {
                RedisClient::open(format!(
                    "redis://{0}:{1}",
                    DEFAULT_REDIS_HOST, DEFAULT_REDIS_PORT
                ))
            })?;

        Ok(SlidingWindowRateLimiter {
            window_size: self.window_size.unwrap_or(DEFAULT_WINDOW_SIZE),
            window_duration: self.window_duration.unwrap_or(DEFAULT_WINDOW_DURATION),
            redis_client,
        })
    }
}

//TODO: tests
