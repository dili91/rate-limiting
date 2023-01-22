use redis::Client as RedisClient;
use std::time::{Duration, Instant, SystemTime};

use crate::{
    entities::{RateLimiterResponse, RequestAllowed, RequestThrottled},
    errors::RateLimiterError,
    RateLimiter,
};

/// Represents a distributed token bucket rate limiter
/// based on [Redis](https://redis.io/)
#[derive(Clone)]
pub struct SlidingWindowRateLimiter {
    /// The size of the sliding window, that is the maximum number of
    /// requests allowed in a single window
    pub window_size: u64,

    /// The duration of the sliding window that the rate limiter takes
    /// into account when deciding whether to allow or throttle a request
    pub window_duration: Duration,

    /// The internal client that will be used to fire requests against Redis
    pub redis_client: RedisClient,
}

/// Implementation of a sliding window rate limiter.
///
/// ## Implementation details
///
/// Implements the sliding window rate liming algorithm, allowing a maximum of `window_size` request
/// in the configured duration defined by the `window_duration` parameter.
///
/// ## Example
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr};
/// use rate_limiter_rs::{factory::RateLimiterFactory, RateLimiter,
///     entities::{RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled}
/// };
///
/// let rate_limiter = RateLimiterFactory::sliding_window()
///     .build()
///     .unwrap();
/// let ip_address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
/// let request_id = RequestIdentifier::Ip(ip_address);
///
/// let rate_limiter_response = rate_limiter.check_request(request_id).unwrap();
///
/// match rate_limiter_response {
///     RateLimiterResponse::RequestAllowed(RequestAllowed {remaining_request_counter}) => {
///         println!("Request allowed! Remaining request counter is {0}.", remaining_request_counter);
///     },
///     RateLimiterResponse::RequestThrottled(RequestThrottled {retry_in}) => {
///         println!("Request throttled! Retry in {0} seconds.", retry_in.as_secs());
///     },
/// }
/// ```
impl RateLimiter for SlidingWindowRateLimiter {
    /// Function that returns the result of the rate limiter checks. Yields an error in case of troubles
    /// connecting to the underlying redis instance.
    ///
    /// ## Implementation details
    /// The implementation of this method heavily relies on Redis commands and [Sorted sets](https://redis.io/docs/data-types/sorted-sets/).
    /// It atomically runs a set commands to:
    ///
    /// 1. Compute the current timestamp, and the start of the current _window_;
    /// 2. Remove all the items (if any) matching the given request identifier and received before the computed window start date;
    /// 3. If not present already, create a sorted set with the given request identifier.
    /// 4. Add the current request to the sorted set as new item having key and value equal to the current timestamp, computed at step one;
    /// 5. Count the number of items in the sorted set, later used to indicate the outstanding request budget, in case the request is allowed;
    /// 6. Retrieve the last request in the updated, valid window and use that to indicate the value of the retry_in information in case the request is throttled.
    /// 7. Set the sorted set to expire in _window_duration_ in seconds.
    ///
    /// The above four commands are wrapped into a Redis [transaction](https://redis.io/docs/manual/transactions/) with the helper provided by the underlying redis crate used.
    /// The combination of `WATCH`, `MULTI` and `EXEC` commands here protect this piece of code from race conditions when multiple
    /// clients are modifying the same key simultaneously.
    ///
    /// Below the output of a MONITOR command on a Redis instance when the `is_request_allowed` function is invoked:
    ///
    /// ```ignore
    /// 1674324083.383248 [0 172.17.0.1:59248] "WATCH" "rl:ip_115.249.235.84"
    /// 1674324083.386649 [0 172.17.0.1:59248] "ZREMRANGEBYSCORE" "rl:ip_115.249.235.84" "-inf" "(1674324023380245000"
    /// 1674324083.386600 [0 172.17.0.1:59248] "MULTI"
    /// 1674324083.386670 [0 172.17.0.1:59248] "ZADD" "rl:ip_115.249.235.84" "NX" "1674324083380245000" "1674324083380245000"
    /// 1674324083.386684 [0 172.17.0.1:59248] "ZCOUNT" "rl:ip_115.249.235.84" "-inf" "+inf"
    /// 1674324083.386698 [0 172.17.0.1:59248] "ZREVRANGEBYSCORE" "rl:ip_115.249.235.84" "+inf" "-inf" "LIMIT" "0" "5"
    /// 1674324083.386712 [0 172.17.0.1:59248] "EXPIRE" "rl:ip_115.249.235.84" "60"
    /// 1674324083.386719 [0 172.17.0.1:59248] "EXEC"
    /// 1674324083.391000 [0 172.17.0.1:59248] "UNWATCH"
    /// 1674324083.395845 [0 172.17.0.1:59250] "WATCH" "rl:ip_115.249.235.84"
    /// 1674324083.398000 [0 172.17.0.1:59250] "MULTI"
    /// 1674324083.398027 [0 172.17.0.1:59250] "ZREMRANGEBYSCORE" "rl:ip_115.249.235.84" "-inf" "(1674324023392739000"
    /// 1674324083.398042 [0 172.17.0.1:59250] "ZADD" "rl:ip_115.249.235.84" "NX" "1674324083392739000" "1674324083392739000"
    /// 1674324083.398054 [0 172.17.0.1:59250] "ZCOUNT" "rl:ip_115.249.235.84" "-inf" "+inf"
    /// 1674324083.398065 [0 172.17.0.1:59250] "ZREVRANGEBYSCORE" "rl:ip_115.249.235.84" "+inf" "-inf" "LIMIT" "0" "5"
    /// 1674324083.398078 [0 172.17.0.1:59250] "EXPIRE" "rl:ip_115.249.235.84" "60"
    /// 1674324083.398084 [0 172.17.0.1:59250] "EXEC"
    /// 1674324083.400599 [0 172.17.0.1:59250] "UNWATCH"
    /// ```
    fn check_request(
        &self,
        request_identifier: crate::entities::RequestIdentifier,
    ) -> Result<crate::entities::RateLimiterResponse, crate::errors::RateLimiterError> {
        let key = &self.build_request_key(request_identifier);

        let mut con = self.redis_client.get_connection()?;

        // Beware that this is NOT monotonic!
        let current_ts = SystemTime::now();

        let current_ts_epoch_time = as_epoch_time(current_ts)?;

        let window_start_ts = current_ts
            .checked_sub(self.window_duration)
            .ok_or(RateLimiterError::ComputeError)?;

        let window_start_epoch_time = as_epoch_time(window_start_ts)?;

        let (request_count, oldest_requests_in_current_window): (u64, Vec<String>) =
            redis::transaction(&mut con, &[key], |con, pipe| {
                pipe.cmd("ZREMRANGEBYSCORE")
                    .arg(key)
                    .arg("-inf")
                    .arg(format!("({}", window_start_epoch_time))
                    .ignore()
                    .cmd("ZADD")
                    .arg(key)
                    .arg("NX")
                    .arg(current_ts_epoch_time as u64)
                    .arg(current_ts_epoch_time as u64)
                    .ignore()
                    .zcount(key, "-inf", "+inf")
                    .cmd("ZREVRANGEBYSCORE")
                    .arg(key)
                    .arg("+inf")
                    .arg("-inf")
                    .arg("LIMIT")
                    .arg("0")
                    .arg("5")
                    .cmd("EXPIRE")
                    .arg(key)
                    .arg(self.window_duration.as_secs())
                    .ignore()
                    .query(con)
            })?;

        let oldest_request_epoch_time: u64 = match oldest_requests_in_current_window.last() {
            Some(l) => l.parse().map_err(|_e| RateLimiterError::ComputeError)?,
            None => 0,
        };

        let response = if request_count <= self.window_size {
            RateLimiterResponse::RequestAllowed(RequestAllowed {
                remaining_request_counter: self.window_size - request_count,
            })
        } else {
            let time_passed_from_first_req =
                Duration::from_nanos(current_ts_epoch_time as u64 - oldest_request_epoch_time);
            let retry_in = self
                .window_duration
                .saturating_sub(time_passed_from_first_req);

            RateLimiterResponse::RequestThrottled(RequestThrottled { retry_in })
        };

        Ok(response)
    }
}

/// Utility method that returns the given timestamp in epoch time, with nanoseconds precision.
fn as_epoch_time(ts: SystemTime) -> Result<u128, crate::RateLimiterError> {
    let epoch_time_nanos = ts
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_e| RateLimiterError::ComputeError)?
        .as_nanos();
    Ok(epoch_time_nanos)
}

#[cfg(test)]
mod test {
    use std::{
        cmp,
        net::{IpAddr, Ipv4Addr},
        time::{Duration, SystemTime},
    };

    use rand::Rng;
    use redis::RedisError;
    use rstest::rstest;
    use uuid::Uuid;

    use crate::{
        builders::RedisSettings, entities::RequestIdentifier, errors::RateLimiterError,
        factory::RateLimiterFactory, RateLimiter,
    };

    use super::as_epoch_time;

    #[rstest]
    #[case::ip(RequestIdentifier::Ip(generate_random_ip()))]
    #[case::custom_id(
        RequestIdentifier::Custom { key: "a_custom_id".to_string(), value: Uuid::new_v4().to_string()  },
    )]
    fn should_yield_a_connection_error(#[case] request_identifier: RequestIdentifier) {
        //arrange
        let rate_limiter = RateLimiterFactory::token_bucket()
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
            RateLimiterError::IoError(RedisError { .. })
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
        let rate_limiter = RateLimiterFactory::sliding_window()
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

    #[test]
    fn as_epoch_time_should_return_current_time() {
        let now = SystemTime::now();

        let now_epoch = as_epoch_time(now);

        assert!(now_epoch.is_ok())
    }

    fn generate_random_ip() -> IpAddr {
        let mut rng = rand::thread_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }
}
