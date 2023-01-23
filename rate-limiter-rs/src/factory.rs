//! Factory pattern for rate limiters. Used by the consumers of this crate.
use crate::builders::{
    fixed_window::FixedWindowRateLimiterBuilder, sliding_window::SlidingWindowRateLimiterBuilder,
};

/// A factory used as entrypoint for building rate limiter variants
/// Based on the selected rate limiter type either a builder object or a simpler
/// object can be returned.
pub struct RateLimiterFactory;

impl RateLimiterFactory {
    /// Provides a builder for a fixed window rate limiter.
    pub fn fixed_window() -> FixedWindowRateLimiterBuilder {
        FixedWindowRateLimiterBuilder::default()
    }

    /// Provides a builder for a sliding window rate limiter.
    pub fn sliding_window() -> SlidingWindowRateLimiterBuilder {
        SlidingWindowRateLimiterBuilder::default()
    }
}
