//! Implementation of a fixed window rate limiter.
//!
//! ## Implementation details
//!
//! Similarly to a classic fixed window rate limiter, this implementation
//! could potentially allow limits overflows if burst of requests are coming
//! along into two adjacent windows.
//!
//! ## Example
//!
//! ```
//! use std::net::{IpAddr, Ipv4Addr};
//! use rate_limiter_rs::{factory::RateLimiterFactory, RateLimiter,
//!     RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled
//! };
//!
//! let rate_limiter = RateLimiterFactory::fixed_window()
//!     .build()
//!     .unwrap();
//! let ip_address = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
//! let request_id = RequestIdentifier::Ip(ip_address);
//!
//! let rate_limiter_response = rate_limiter.check_request(request_id).unwrap();
//!
//! match rate_limiter_response {
//!     RateLimiterResponse::RequestAllowed(RequestAllowed {remaining_request_counter}) => {
//!         println!("Request allowed! Remaining request counter is {0}.", remaining_request_counter);
//!     },
//!     RateLimiterResponse::RequestThrottled(RequestThrottled {retry_in}) => {
//!         println!("Request throttled! Retry in {0} seconds.", retry_in.as_secs());
//!     },
//! }
//! ```
use std::time::Duration;

use redis::Client as RedisClient;

use crate::{
    errors::RateLimiterError, RateLimiter, RateLimiterResponse, RequestAllowed, RequestIdentifier,
    RequestThrottled,
};

/// Represents a distributed fixed windowå rate limiter
/// based on [Redis](https://redis.io/)
#[derive(Clone)]
pub struct FixedWindowRateLimiter {
    /// The size of the window, that is the maximum number
    /// of requests that the rate limiter will allow for a time equal to the _window_duration_
    pub window_size: u64,

    /// Represents how long the window should be considered valid.
    /// This can be considered as the equivalent of the _refill rate_
    pub window_validity: Duration,

    /// The internal client that will be used to fire requests against Redis
    pub redis_client: RedisClient,
}

impl RateLimiter for FixedWindowRateLimiter {
    /// Function that returns the result of the rate limiter checks. Yields an error in case of troubles
    /// connecting to the underlying redis instance.
    ///
    /// ## Implementation details
    /// The implementation of this method heavily relies on Redis commands. It atomically runs a set commands to:
    ///
    /// 1. Create a key, representing the rate limiter, if not existing, with value
    /// equals to 0;
    /// 2. Set the configured expiration on it, if not set already;
    /// 3. Increase by 1 the existing counter;
    /// 4. Get the updated expiry of the rate limiter.
    ///
    /// The above four commands are wrapped into a Redis [transaction](https://redis.io/docs/manual/transactions/) with the helper provided by the underlying redis crate used.
    /// The combination of `WATCH`, `MULTI` and `EXEC` commands here protect this piece of code from race conditions when multiple
    /// clients are modifying the same key simultaneously.
    ///
    /// Below the output of a MONITOR command on a Redis instance when the `is_request_allowed` function is invoked:
    ///
    /// ```ignore
    /// 1675511728.833664 [0 172.28.0.5:48922] "WATCH" "rl:ip_172.28.0.6"
    /// 1675511728.834677 [0 172.28.0.5:48922] "MULTI"
    /// 1675511728.835237 [0 172.28.0.5:48922] "SETNX" "rl:ip_172.28.0.6" "0"
    /// 1675511728.835358 [0 172.28.0.5:48922] "EXPIRE" "rl:ip_172.28.0.6" "60" "NX"
    /// 1675511728.835455 [0 172.28.0.5:48922] "INCRBY" "rl:ip_172.28.0.6" "1"
    /// 1675511728.835526 [0 172.28.0.5:48922] "TTL" "rl:ip_172.28.0.6"
    /// 1675511728.835626 [0 172.28.0.5:48922] "EXEC"
    /// 1675511728.836371 [0 172.28.0.5:48922] "UNWATCH"
    /// ```
    fn check_request(
        &self,
        request_identifier: RequestIdentifier,
    ) -> Result<RateLimiterResponse, RateLimiterError> {
        let key = &self.build_request_key(request_identifier);

        let mut con = self.redis_client.get_connection()?;

        let (executed_request_counter, expire_in_seconds): (u64, u64) =
            redis::transaction(&mut con, &[key], |con, pipe| {
                pipe.cmd("SETNX")
                    .arg(key)
                    .arg(0)
                    .ignore()
                    .cmd("EXPIRE")
                    .arg(key)
                    .arg(self.window_validity.as_secs())
                    .arg("NX")
                    .ignore()
                    .cmd("INCRBY")
                    .arg(key)
                    .arg(1)
                    .cmd("TTL")
                    .arg(key)
                    .query(con)
            })?;

        let response = if executed_request_counter <= self.window_size {
            RateLimiterResponse::RequestAllowed(RequestAllowed {
                remaining_request_counter: self.window_size - executed_request_counter as u64,
            })
        } else {
            RateLimiterResponse::RequestThrottled(RequestThrottled {
                retry_in: Duration::from_secs(expire_in_seconds),
            })
        };

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use std::{
        cmp,
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };

    use rand::Rng;
    use rstest::rstest;
    use uuid::Uuid;

    use crate::{
        builders::RedisSettings, errors::RateLimiterError, factory::RateLimiterFactory,
        RateLimiter, RequestIdentifier,
    };

    #[rstest]
    #[case::ip(RequestIdentifier::Ip(generate_random_ip()))]
    #[case::custom_id(
        RequestIdentifier::Custom { key: "a_custom_id".to_string(), value: Uuid::new_v4().to_string()  },
    )]
    fn should_yield_a_connection_error(#[case] request_identifier: RequestIdentifier) {
        //arrange
        let rate_limiter = RateLimiterFactory::fixed_window()
            .with_redis_settings(RedisSettings {
                host: "whatever".to_string(),
                port: 1,
            })
            .build()
            .unwrap();
        let _ip = generate_random_ip();

        //act
        let res = rate_limiter.check_request(request_identifier);

        //assert
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            RateLimiterError::IoError(redis::RedisError { .. })
        ))
    }

    #[rstest]
    #[case::ip(RequestIdentifier::Ip(generate_random_ip()))]
    #[case::custom_id(
        RequestIdentifier::Custom { key: "a_custom_id".to_string(), value: Uuid::new_v4().to_string() },
    )]
    fn should_check_request_eligibility(#[case] request_identifier: RequestIdentifier) {
        //arrange
        let window_size = 5;
        let window_duration = Duration::from_secs(60);
        let rate_limiter = RateLimiterFactory::fixed_window()
            .with_window_size(window_size)
            .with_window_duration(window_duration)
            .build()
            .unwrap();

        for n in 1..=2 * window_size {
            //act
            let res = rate_limiter
                .check_request(request_identifier.clone())
                .unwrap();

            if n <= window_size {
                let allowed_res = res.as_allowed();
                assert_eq!(
                    allowed_res.remaining_request_counter,
                    cmp::max(0, window_size as i64 - n as i64) as u64
                )
            } else {
                let tolerance_secs = window_duration.as_secs() * 5 / 100;
                let throttled_res = res.as_throttled();
                let retry_in_secs = throttled_res.retry_in.as_secs();
                assert!(
                    retry_in_secs > 0 && retry_in_secs <= window_duration.as_secs(),
                    "retry in is not in valid range"
                );
                assert!(
                    window_duration.as_secs() - throttled_res.retry_in.as_secs() <= tolerance_secs,
                    "retry_in suggestion is greater than tolerance of {0}s",
                    tolerance_secs
                )
            }
        }
    }

    fn generate_random_ip() -> IpAddr {
        let mut rng = rand::thread_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }
}
