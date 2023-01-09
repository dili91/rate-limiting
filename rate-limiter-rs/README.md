# rate-limiter-rs

A distributed rate limiter component written in Rust, that follows a slightly revised version of the Token bucket algorithm.

## Implementation details

Unlike traditional token bucket rate limiters, this implementation does not refill the bucket at a fixed interval rate, but it creates a bucket on the very first request belonging to the same ip address and expire the bucket after a configured deadline. This approach has the advantage that the same origin cannot fire more than the maximum allowed requests in a period which is across 2 adjacent windows.

## Requirements

As of today, Redis is a strict requirement of this library.

## TODO
- [ ] Github workflow 

## Areas of improvements
- [ ] More flexibility, right now the implementation is coupled to Redis
- [ ] Support for custom source/origin identifiers and not just IP addresses
- [ ] Add support for async
- [ ] Add support for TLS