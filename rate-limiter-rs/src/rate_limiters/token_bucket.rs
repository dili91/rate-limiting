use std::time::Duration;

use redis::Client as RedisClient;

use crate::{
    entities::{RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled},
    errors::RateLimiterError,
    RateLimiter,
};

/// Represents a distributed token bucket rate limiter
/// based on [Redis](https://redis.io/)
#[derive(Clone)]
pub struct TokenBucketRateLimiter {
    /// The size of the bucket, that is the maximum number
    /// of requests that the rate limiter will allow for a time equal to the _bucket_validity_
    pub bucket_size: u64,

    /// Represents how long the bucket should be considered valid.
    /// This can be considered as the equivalent of the _refill rate_
    pub bucket_validity: Duration,

    /// The internal client that will be used to fire requests against Redis
    pub redis_client: RedisClient,
}

/// Implementation of a revised token bucket rate limiter.
///
/// ## Implementation details
///
/// Unlike traditional token bucket rate limiters, this implementation does not
/// refill the bucket at a predefined interval rate, but it creates a bucket on the
/// very first request belonging to the same ip address, or a custom origin
/// identifier, with a configured expiry time.
///
/// Compared to a classic token bucket implementation
/// such approach should be slightly more efficient with regards to memory
/// consumption, as we have buckets held in memory just for those IP addresses
/// (or origin identifiers) that actually hit our service.
///
/// The disadvantage of this solution compared to a canonical token bucket
/// rate limiter is that once the request budget is reached, the caller
/// should wait the bucket expiration before firing any new request, as
/// opposed to having the request budget bumped of one (or more) token
/// at a regular interval, typically 1 or few seconds.
///
/// Just like any other Token Bucket implementation it does not prevent bursts of requests
/// happening in adjacent windows.
///
/// ## Example
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr};
/// use rate_limiter_rs::{factory::RateLimiterFactory, RateLimiter,
///     entities::{RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled}
/// };
///
/// let rate_limiter = RateLimiterFactory::token_bucket()
///     .build()
///     .unwrap();
/// let ip_address = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
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
impl RateLimiter for TokenBucketRateLimiter {
    /// Function that returns the result of the rate limiter checks. Yields an error in case of troubles
    /// connecting to the underlying redis instance.
    ///
    /// ## Implementation details
    /// The implementation of this method heavily relies on Redis commands. It atomically runs a set commands to:
    ///
    /// 1. Create a key, representing the token bucket, if not existing;
    /// 2. Set the configured expiration on it, if not set already;
    /// 3. Decrease by 1 the existing counter;
    /// 4. Get the updated expiry of the bucket.
    ///
    /// The above four commands are wrapped into a Redis [transaction](https://redis.io/docs/manual/transactions/) with the helper provided by the underlying redis crate used.
    /// The combination of `WATCH`, `MULTI` and `EXEC` commands here protect this piece of code from race conditions when multiple
    /// clients are modifying the same key simultaneously.
    ///
    /// Below the output of a MONITOR command on a Redis instance when the `is_request_allowed` function is invoked:
    ///
    /// ```ignore
    /// 1673481594.796098 [0 192.168.224.4:53504] "WATCH" "rl:ip_192.168.224.6"
    /// 1673481594.796875 [0 192.168.224.4:53504] "MULTI"
    /// 1673481594.796902 [0 192.168.224.4:53504] "SETNX" "rl:ip_192.168.224.6" "5"
    /// 1673481594.796916 [0 192.168.224.4:53504] "EXPIRE" "rl:ip_192.168.224.6" "60" "NX"
    /// 1673481594.796929 [0 192.168.224.4:53504] "INCRBY" "rl:ip_192.168.224.6" "-1"
    /// 1673481594.796941 [0 192.168.224.4:53504] "TTL" "rl:ip_192.168.224.6"
    /// 1673481594.796950 [0 192.168.224.4:53504] "EXEC"
    /// 1673481594.799002 [0 192.168.224.4:53504] "UNWATCH"
    /// ```
    fn check_request(
        &self,
        request_identifier: RequestIdentifier,
    ) -> Result<RateLimiterResponse, RateLimiterError> {
        let key = &self.build_request_key(request_identifier);

        let mut con = self.redis_client.get_connection()?;

        let (remaining_tokens, expire_in_seconds): (i64, u64) =
            redis::transaction(&mut con, &[key], |con, pipe| {
                pipe.cmd("SETNX")
                    .arg(key)
                    .arg(self.bucket_size)
                    .ignore()
                    .cmd("EXPIRE")
                    .arg(key)
                    .arg(self.bucket_validity.as_secs())
                    .arg("NX")
                    .ignore()
                    .cmd("INCRBY")
                    .arg(key)
                    .arg(-1)
                    .cmd("TTL")
                    .arg(key)
                    .query(con)
            })?;

        let response = if remaining_tokens >= 0 {
            RateLimiterResponse::RequestAllowed(RequestAllowed {
                remaining_request_counter: remaining_tokens as u64,
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
        builders::RedisSettings, entities::RequestIdentifier, errors::RateLimiterError,
        factory::RateLimiterFactory, RateLimiter,
    };

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
        let bucket_size = 5;
        let bucket_validity = Duration::from_secs(60);
        let rate_limiter = RateLimiterFactory::token_bucket()
            .with_bucket_size(bucket_size)
            .with_bucket_validity(bucket_validity)
            .build()
            .unwrap();

        for n in 1..=2 * bucket_size {
            //act
            let res = rate_limiter
                .check_request(request_identifier.clone())
                .unwrap();

            if n <= bucket_size {
                let allowed_res = res.as_allowed();
                assert_eq!(
                    allowed_res.remaining_request_counter,
                    cmp::max(0, bucket_size as i64 - n as i64) as u64
                )
            } else {
                let tolerance_secs = bucket_validity.as_secs() * 5 / 100;
                let throttled_res = res.as_throttled();
                let retry_in_secs = throttled_res.retry_in.as_secs();
                assert!(
                    retry_in_secs > 0 && retry_in_secs <= bucket_validity.as_secs(),
                    "retry in is not in valid range"
                );
                assert!(
                    bucket_validity.as_secs() - throttled_res.retry_in.as_secs() <= tolerance_secs,
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
