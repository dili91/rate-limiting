use std::time::Duration;

use entities::{
    RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled,
    TokenBucketRateLimiter,
};
use errors::RateLimiterError;

pub mod builders;
pub mod entities;
pub mod errors;
pub mod factory;

/// Trait representing the capabilities offered by the rate limiter
pub trait RateLimiter {
    fn check_request(
        &self,
        request_identifier: RequestIdentifier,
    ) -> Result<RateLimiterResponse, RateLimiterError>;
}

impl TokenBucketRateLimiter {
    /// method that builds a request key based on the different input
    fn build_request_key(&self, request_identifier: RequestIdentifier) -> String {
        match request_identifier {
            RequestIdentifier::Ip(ip) => format!("rl:ip_{}", ip),
            RequestIdentifier::Custom { key, value } => format!("rl:cst_{0}:{1}", key, value),
        }
    }
}

/// Implementation of a revised token bucket rate limiter.
///
/// ## Implementation details
///
/// Unlike traditional token bucket rate limiters, this implementation does not refill the bucket at a fixed interval rate, but it creates a bucket
/// on the very first request belonging to the same ip address and expire the bucket after a configured deadline.
/// This approach has the advantage that the same origin cannot fire more than the maximum allowed
/// requests in a period which is across 2 adjacent windows.
///
/// ## Example
/// ```
/// use std::net::{IpAddr, Ipv4Addr};
/// use crate::rate_limiter_rs::RateLimiter;
/// use crate::rate_limiter_rs::builder::RateLimiterBuilder;
///
/// let rate_limiter = RateLimiterBuilder::default().build().unwrap();
/// let ip_address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
///
/// let rate_limiter_response = rate_limiter.is_request_allowed(ip_address).unwrap();
///
/// assert!(rate_limiter_response.is_request_allowed);
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
    /// ```
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
    #[case::ip(
        RequestIdentifier::Ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        "rl:ip_1.2.3.4"
    )]
    #[case::custom_id(
        RequestIdentifier::Custom { key: "client_id".to_string(), value: "dili91".to_string() },
        "rl:cst_client_id:dili91"
    )]
    fn should_build_request_identifier(
        #[case] request_identifier: RequestIdentifier,
        #[case] expected_key: &str,
    ) {
        let rate_limiter = RateLimiterFactory::fixed_token_bucket().build().unwrap();

        assert_eq!(
            rate_limiter.build_request_key(request_identifier),
            expected_key
        )
    }

    // #[test]
    #[rstest]
    #[case::ip(RequestIdentifier::Ip(generate_random_ip()))]
    #[case::custom_id(
        RequestIdentifier::Custom { key: "a_custom_id".to_string(), value: Uuid::new_v4().to_string()  },
    )]
    fn should_yield_a_connection_error(#[case] request_identifier: RequestIdentifier) {
        //arrange
        let rate_limiter = RateLimiterFactory::fixed_token_bucket()
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
            RateLimiterError::ConnectionError
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
        let refill_rate = Duration::from_secs(60);
        let rate_limiter = RateLimiterFactory::fixed_token_bucket()
            .with_bucket_size(bucket_size)
            .with_bucket_validity(refill_rate)
            .build()
            .unwrap();

        for n in 1..=10 as u64 {
            //act
            let res = rate_limiter
                .check_request(request_identifier.clone())
                .unwrap();

            if n <= bucket_size {
                assert_eq!(
                    res.as_allowed().remaining_request_counter,
                    cmp::max(0, bucket_size as i64 - n as i64) as u64
                )
            } else {
                assert!(res.as_throttled().retry_in.as_secs() > 0)
            }
        }
    }

    fn generate_random_ip() -> IpAddr {
        let mut rng = rand::thread_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }
}
