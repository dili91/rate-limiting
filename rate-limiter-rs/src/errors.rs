use redis::RedisError;

/// Enum that represent the error potentially returned by the rate limiter component
#[derive(Debug)]
pub enum RateLimiterError {
    InitError,
    ConnectionError,
}

// Converts from RedisError to our custom errors
impl From<RedisError> for RateLimiterError {
    fn from(redis_error: RedisError) -> Self {
        match redis_error.kind() {
            redis::ErrorKind::InvalidClientConfig => RateLimiterError::InitError,
            _ => RateLimiterError::ConnectionError,
        }
    }
}
