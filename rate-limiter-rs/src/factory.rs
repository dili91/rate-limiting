//! Factory pattern for rate limiters. Used by the consumers of this crate.
use crate::builders::{
    sliding_window::SlidingWindowRateLimiterBuilder, token_bucket::TokenBucketRateLimiterBuilder,
};

/// A factory used as entrypoint for building rate limiter variants
/// Based on the selected rate limiter type either a builder object or a simpler
/// object can be returned.
pub struct RateLimiterFactory;

impl RateLimiterFactory {
    /// Provides a builder for a token bucket rate limiter.
    pub fn token_bucket() -> TokenBucketRateLimiterBuilder {
        TokenBucketRateLimiterBuilder::default()
    }

    pub fn sliding_window() -> SlidingWindowRateLimiterBuilder {
        SlidingWindowRateLimiterBuilder::default()
    }
}
