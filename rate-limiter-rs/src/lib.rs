use std::{net::IpAddr, time::Duration};

use errors::RateLimiterError;

pub mod builders;
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

/// Struct for requests that are allowed by the rate limiter
#[derive(Debug)]
pub struct RequestAllowed {
    /// the updated counter of available requests for the given ip/custom request id
    pub remaining_request_counter: u64,
}

/// Struct for requests that are throttled by the rate limiter
#[derive(Debug)]
pub struct RequestThrottled {
    /// a duration representing when the user should retry the request
    pub retry_in: Duration,
}

/// Wrapper enum that describes the list of possible responses returned by the rate limiter
/// with each specific inner detail according to the scenario
#[derive(Debug)]
pub enum RateLimiterResponse {
    /// variant for requests that are allowed
    RequestAllowed(RequestAllowed),
    /// variant for requests that are throttled
    RequestThrottled(RequestThrottled),
}

/// enum that represents the possible input types for our rate limiter
#[derive(Clone)]
pub enum RequestIdentifier {
    /// An Ip address. Used when we want to rate limit requests based on the Ip address
    /// from which the request was fired
    Ip(IpAddr),
    /// A custom identifier in a string format. Used when we want to rate limit based on
    /// custom criteria, like a client identifier.
    Custom { key: String, value: String },
}

/// Utility method used in tests only
#[cfg(test)]
impl RateLimiterResponse {
    pub fn as_allowed(self) -> RequestAllowed {
        if let RateLimiterResponse::RequestAllowed(r) = self {
            r
        } else {
            panic!("RequestThrottled variant!")
        }
    }
    pub fn as_throttled(self) -> RequestThrottled {
        if let RateLimiterResponse::RequestThrottled(r) = self {
            r
        } else {
            panic!("RequestAllowed variant!")
        }
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use rstest::rstest;

    use crate::{factory::RateLimiterFactory, RateLimiter, RequestIdentifier};

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
        let rate_limiter = RateLimiterFactory::token_bucket().build().unwrap();

        assert_eq!(
            rate_limiter.build_request_key(request_identifier),
            expected_key
        )
    }
}
