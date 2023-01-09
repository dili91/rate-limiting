use std::{net::IpAddr, time::Duration};

use entities::{RateLimiterResponse, TokenBucketRateLimiter};
use errors::RateLimiterError;

pub mod builder;
pub mod entities;
pub mod errors;

/// Trait representing the capabilities offered by the rate limiter
pub trait RateLimiter {
    fn is_request_allowed(
        &self,
        ip_address: IpAddr,
    ) -> Result<RateLimiterResponse, RateLimiterError>;
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
    /// 1673395296.009154 [0 192.168.192.5:35814] "WATCH" "192.168.192.6"
    /// 1673395296.009712 [0 192.168.192.5:35814] "MULTI"
    /// 1673395296.012079 [0 192.168.192.5:35814] "SETNX" "192.168.192.6" "5"
    /// 1673395296.012161 [0 192.168.192.5:35814] "EXPIRE" "192.168.192.6" "60" "NX"
    /// 1673395296.012200 [0 192.168.192.5:35814] "INCRBY" "192.168.192.6" "-1"
    /// 1673395296.012673 [0 192.168.192.5:35814] "TTL" "192.168.192.6"
    /// 1673395296.012724 [0 192.168.192.5:35814] "EXEC"
    /// 1673395296.013392 [0 192.168.192.5:35814] "UNWATCH"
    /// ```
    fn is_request_allowed(
        &self,
        ip_address: IpAddr,
    ) -> Result<RateLimiterResponse, RateLimiterError> {
        let key = &ip_address.to_string();

        let mut con = self.redis_client.get_connection()?;

        let (remaining_tokens, expire_in_seconds): (isize, u64) =
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

        Ok(RateLimiterResponse {
            remaining_request_counter: remaining_tokens,
            is_request_allowed: remaining_tokens >= 0,
            expire_in: Duration::from_secs(expire_in_seconds),
        })
    }
}

#[cfg(test)]
mod test {
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };

    use rand::Rng;

    use crate::{
        builder::{RateLimiterBuilder, RedisSettings},
        errors::RateLimiterError,
        RateLimiter,
    };

    #[test]
    fn should_yield_a_connection_error() {
        //arrange
        let rate_limiter = RateLimiterBuilder::default()
            .with_redis_settings(RedisSettings {
                host: "whatever".to_string(),
                port: 1,
            })
            .build()
            .unwrap();
        let ip = generate_random_ip();

        //act
        let res = rate_limiter.is_request_allowed(ip);

        //assert
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            RateLimiterError::ConnectionError
        ))
    }

    #[test]
    fn should_check_request_eligibility() {
        //arrange
        let bucket_size = 5;
        let refill_rate = Duration::from_secs(60);
        let rate_limiter = RateLimiterBuilder::default()
            .with_bucket_size(bucket_size)
            .with_bucket_validity(refill_rate)
            .build()
            .unwrap();
        let ip = generate_random_ip();

        for n in 1..=10 {
            //act
            let res = rate_limiter.is_request_allowed(ip).unwrap();

            //assert
            assert!(res.expire_in.as_secs() > 0);
            assert_eq!(res.remaining_request_counter, bucket_size as isize - n);
            assert_eq!(res.is_request_allowed, n <= bucket_size as isize);
        }
    }

    fn generate_random_ip() -> IpAddr {
        let mut rng = rand::thread_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }
}
