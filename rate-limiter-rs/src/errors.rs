use redis::RedisError;

/// Enum that represent the error potentially returned by the rate limiter component
#[derive(thiserror::Error, Debug)]
pub enum RateLimiterError {
    #[error("Init error")]
    InitError(#[source] RedisError),
    #[error("Compute error")]
    ComputeError,
    #[error("Connect error: {0}")]
    IoError(#[source] RedisError),
}

// Converts from RedisError to our custom errors
impl From<RedisError> for RateLimiterError {
    fn from(redis_error: RedisError) -> Self {
        match redis_error.kind() {
            redis::ErrorKind::InvalidClientConfig => RateLimiterError::InitError(redis_error),
            _ => RateLimiterError::IoError(redis_error),
        }
    }
}
