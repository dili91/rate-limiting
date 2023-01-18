use redis::Client as RedisClient;
use std::time::{Duration, SystemTime};

use crate::{
    entities::{RateLimiterResponse, RequestAllowed, RequestThrottled},
    errors::RateLimiterError,
    RateLimiter,
};

#[derive(Clone)]
pub struct SlidingWindowRateLimiter {
    pub window_size: u64,
    pub window_duration: Duration,
    pub redis_client: RedisClient,
}

impl SlidingWindowRateLimiter {
    fn as_epoch_time(&self, ts: SystemTime) -> Result<u64, crate::RateLimiterError> {
        let epoch_time_nanos = ts
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_e| RateLimiterError::ComputeError)?
            .as_nanos();
        //TODO:review
        Ok(u64::try_from(epoch_time_nanos).map_err(|_e| RateLimiterError::ComputeError)?)
    }
}

impl RateLimiter for SlidingWindowRateLimiter {
    fn check_request(
        &self,
        request_identifier: crate::entities::RequestIdentifier,
    ) -> Result<crate::entities::RateLimiterResponse, crate::errors::RateLimiterError> {
        let key = &self.build_request_key(request_identifier);

        let mut con = self.redis_client.get_connection()?;

        // TODO: document this is not monotonic
        let current_ts = SystemTime::now();

        let current_ts_epoch_time = self.as_epoch_time(current_ts)?;

        let window_start_ts = current_ts
            .checked_sub(self.window_duration)
            .ok_or(RateLimiterError::ComputeError)?;

        let window_start_epoch_time = self.as_epoch_time(window_start_ts)?;

        //TODO: add TTL -> save some memory
        let (request_count, oldest_request_in_updated_window_epoch_time): (u64, Vec<String>) =
            redis::transaction(&mut con, &[key], |con, pipe| {
                pipe.cmd("ZREMRANGEBYSCORE")
                    .arg(key)
                    .arg("-inf")
                    .arg(format!("({}", window_start_epoch_time))
                    .ignore()
                    .cmd("ZADD")
                    .arg(key)
                    .arg("NX")
                    .arg(current_ts_epoch_time)
                    .arg(current_ts_epoch_time)
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

        println!(
            "request_count={} most_recent={:?}",
            request_count, oldest_request_in_updated_window_epoch_time
        );

        //TODO: if let / else ?
        let oldest_request_in_updated_window_epoch_time: u64 =
            match oldest_request_in_updated_window_epoch_time.last() {
                Some(l) => l.parse().map_err(|_e| RateLimiterError::ComputeError)?,
                None => 0,
            };

        let response = if request_count <= self.window_size {
            RateLimiterResponse::RequestAllowed(RequestAllowed {
                remaining_request_counter: self.window_size - request_count,
            })
        } else {
            //let retry_in = current_ts_epoch_time - mos    t_recent_entry_epoch_time;
            let time_passed_from_first_req = Duration::from_nanos(
                current_ts_epoch_time - oldest_request_in_updated_window_epoch_time,
            );
            let retry_in = self
                .window_duration
                .saturating_sub(time_passed_from_first_req);

            RateLimiterResponse::RequestThrottled(RequestThrottled { retry_in })
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
        assert!(matches!(res.unwrap_err(), RateLimiterError::IoError(_))) //TODO: improve assertion
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
                assert_eq!(
                    res.as_allowed().remaining_request_counter,
                    cmp::max(0, window_size as i64 - n as i64) as u64
                )
            } else {
                let retry = res.as_throttled().retry_in.as_secs();
                println!("Retry in seconds: {}", retry);
                assert!(retry > 0)
            }
        }
    }

    fn generate_random_ip() -> IpAddr {
        let mut rng = rand::thread_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }
}
