use crate::builders::TokenBucketRateLimiterBuilder;

/// A factory used as entrypoint for building rate limiter variants
/// Based on the selected rate limiter type either a builder object or a simpler
/// object can be returned.
pub struct RateLimiterFactory;

impl RateLimiterFactory {
    /// Provides a builder for a token bucket rate limiter.
    pub fn fixed_token_bucket() -> TokenBucketRateLimiterBuilder {
        TokenBucketRateLimiterBuilder::default()
    }
}
