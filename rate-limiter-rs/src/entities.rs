use std::{net::IpAddr, time::Duration};

//TODO: merge into lib.rs ?

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
