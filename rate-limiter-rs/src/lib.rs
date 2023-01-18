use entities::{RateLimiterResponse, RequestIdentifier};
use errors::RateLimiterError;

pub mod builders;
pub mod entities;
pub mod errors;
pub mod factory;
pub mod rate_limiters;

/// Trait representing the capabilities offered by the rate limiter
pub trait RateLimiter {
    /// method that builds a request key based on the different input
    fn build_request_key(&self, request_identifier: RequestIdentifier) -> String {
        match request_identifier {
            RequestIdentifier::Ip(ip) => format!("rl:ip_{}", ip),
            RequestIdentifier::Custom { key, value } => format!("rl:cst_{0}:{1}", key, value),
        }
    }

    fn check_request(
        &self,
        request_identifier: RequestIdentifier,
    ) -> Result<RateLimiterResponse, RateLimiterError>;
}

//TODO: remove _fixed_ from token bucket

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use rstest::rstest;

    use crate::{entities::RequestIdentifier, factory::RateLimiterFactory, RateLimiter};

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
}
