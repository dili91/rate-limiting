use std::time::Duration;

use redis::Client;

use crate::{errors::RateLimiterError, TokenBucketRateLimiter};

const DEFAULT_BUCKET_SIZE: usize = 5;
const DEFAULT_BUCKET_VALIDITY: Duration = Duration::from_secs(60);
const DEFAULT_REDIS_HOST: &str = "127.0.0.1";
const DEFAULT_REDIS_PORT: u16 = 6379;

/// Builder component for a rate limiter instance. It accepts the bucket size and validity,
/// as well as the underlying redis configurations. All values are optional and defaults are
/// applied if not explicitly specified by the user.
#[derive(Default)]
pub struct RateLimiterBuilder {
    /// The size of the bucket, that is the maximum number
    /// of requests that the rate limiter will allow for a time equal to the _bucket_validity_
    bucket_size: Option<usize>,

    /// Represents how long the bucket should be considered valid.
    /// This can be considered as the equivalent of the _refill rate_
    bucket_validity: Option<Duration>,

    /// The configuration of the underlying [Redis](https://redis.io) server used
    redis_settings: Option<RedisSettings>,
}

/// Represent the Redis configuration object
pub struct RedisSettings {
    /// The host of the Redis server used.
    pub host: String,
    /// The port of the Redis server used.
    pub port: u16,
}

impl Default for RedisSettings {
    fn default() -> Self {
        Self {
            host: DEFAULT_REDIS_HOST.to_string(),
            port: DEFAULT_REDIS_PORT,
        }
    }
}

impl RateLimiterBuilder {
    /// Setter for the rate limiter bucket size.
    pub fn with_bucket_size(mut self, size: usize) -> Self {
        self.bucket_size = Some(size);
        self
    }

    /// Setter for the rate limiter bucket validity.
    pub fn with_bucket_validity(mut self, refill_rate: Duration) -> Self {
        self.bucket_validity = Some(refill_rate);
        self
    }

    /// Setter for the underlying Redis server settings.
    pub fn with_redis_settings(mut self, redis_settings: RedisSettings) -> Self {
        self.redis_settings = Some(redis_settings);
        self
    }

    /// Function that tries to build the rate limiter.
    pub fn build(&self) -> Result<TokenBucketRateLimiter, RateLimiterError> {
        let redis_client = self
            .redis_settings
            .as_ref()
            .map(|rs| Client::open(format!("redis://{0}:{1}", rs.host, rs.port)))
            .unwrap_or_else(|| {
                Client::open(format!(
                    "redis://{0}:{1}",
                    DEFAULT_REDIS_HOST, DEFAULT_REDIS_PORT
                ))
            })?;

        Ok(TokenBucketRateLimiter {
            bucket_size: self.bucket_size.unwrap_or(DEFAULT_BUCKET_SIZE),
            bucket_validity: self.bucket_validity.unwrap_or(DEFAULT_BUCKET_VALIDITY),
            redis_client,
        })
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::builder::{
        RedisSettings, DEFAULT_BUCKET_SIZE, DEFAULT_BUCKET_VALIDITY, DEFAULT_REDIS_HOST,
        DEFAULT_REDIS_PORT,
    };

    use super::RateLimiterBuilder;

    #[test]
    fn should_build_rate_limiter_with_default_options() {
        let rate_limiter = RateLimiterBuilder::default().build().unwrap();

        assert_eq!(rate_limiter.bucket_size, DEFAULT_BUCKET_SIZE);
        assert_eq!(rate_limiter.bucket_validity, DEFAULT_BUCKET_VALIDITY);
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
        let bucket_size = 3;
        let bucket_validity = Duration::from_secs(15);
        let redis_host = "redis".to_string();
        let redis_port = 1234;
        let rate_limiter = RateLimiterBuilder::default()
            .with_bucket_size(bucket_size)
            .with_bucket_validity(bucket_validity)
            .with_redis_settings(RedisSettings {
                host: redis_host.clone(),
                port: redis_port,
            })
            .build()
            .unwrap();

        assert_eq!(rate_limiter.bucket_size, bucket_size);
        assert_eq!(rate_limiter.bucket_validity, bucket_validity);
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
