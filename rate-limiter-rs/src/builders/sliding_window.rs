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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::builders::{
        sliding_window::{
            SlidingWindowRateLimiterBuilder, DEFAULT_WINDOW_DURATION, DEFAULT_WINDOW_SIZE,
        },
        RedisSettings, DEFAULT_REDIS_HOST, DEFAULT_REDIS_PORT,
    };

    #[test]
    fn should_build_rate_limiter_with_default_options() {
        let rate_limiter = SlidingWindowRateLimiterBuilder::default().build().unwrap();

        assert_eq!(rate_limiter.window_size, DEFAULT_WINDOW_SIZE);
        assert_eq!(rate_limiter.window_duration, DEFAULT_WINDOW_DURATION);
        assert_eq!(
            rate_limiter
                .redis_client
                .get_connection_info()
                .addr
                .to_string(),
            format!("{0}:{1}", DEFAULT_REDIS_HOST, DEFAULT_REDIS_PORT)
        )
    }

    #[test]
    fn should_build_rate_limiter_with_custom_options() {
        let window_size = 3;
        let window_duration = Duration::from_secs(15);
        let redis_host = "redis".to_string();
        let redis_port = 1234;
        let rate_limiter = SlidingWindowRateLimiterBuilder::default()
            .with_window_size(window_size)
            .with_window_duration(window_duration)
            .with_redis_settings(RedisSettings {
                host: redis_host.clone(),
                port: redis_port,
            })
            .build()
            .unwrap();

        assert_eq!(rate_limiter.window_size, window_size);
        assert_eq!(rate_limiter.window_duration, window_duration);
        assert_eq!(
            rate_limiter
                .redis_client
                .get_connection_info()
                .addr
                .to_string(),
            format!("{0}:{1}", redis_host, redis_port)
        )
    }
}
