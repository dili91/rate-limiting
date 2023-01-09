use std::time::Duration;

use redis::Client as RedisClient;

/// Represents a distributed token bucket rate limiter
/// based on [Redis](https://redis.io/)
#[derive(Clone)]
pub struct TokenBucketRateLimiter {
    /// The size of the bucket, that is the maximum number
    /// of requests that the rate limiter will allow for a time equal to the _bucket_validity_
    pub(crate) bucket_size: usize,

    /// Represents how long the bucket should be considered valid.
    /// This can be considered as the equivalent of the _refill rate_
    pub(crate) bucket_validity: Duration,

    /// The internal client that will be used to fire requests against Redis
    pub(crate) redis_client: RedisClient,
}

/// Represents the rate limiter response, that includes whether a request is (was) allowed or not,
/// as well as the information about the request budget and the duration of the current token bucket.
#[derive(Debug)]
pub struct RateLimiterResponse {
    /// A boolean representing whether the request is allowed or not
    pub is_request_allowed: bool,

    /// a counter that represent the number of requests that are still available for
    /// the _expire_in_ duration. If `is_request_allowed` is false, this counter is negative and represents to number
    /// of requests that exceeded the allowed request limit
    pub remaining_request_counter: isize,

    /// Represents how long the bucket will be
    pub expire_in: Duration,
}
