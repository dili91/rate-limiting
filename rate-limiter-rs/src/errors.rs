use redis::RedisError;

/// Enum that represent the error potentially returned by the rate limiter component
#[derive(Debug)]
pub enum RateLimiterError {
    InitError,
    ComputeError,
    ConnectError,
}

// Converts from RedisError to our custom errors
impl From<RedisError> for RateLimiterError {
    fn from(redis_error: RedisError) -> Self {
        //TODO: include stack trace in error.
        //TODO: differentiate redis query vs connection error
        eprintln!("redis error: {:?}", redis_error);
        match redis_error.kind() {
            redis::ErrorKind::InvalidClientConfig => RateLimiterError::InitError,
            _ => RateLimiterError::ConnectError,
        }
    }
}
