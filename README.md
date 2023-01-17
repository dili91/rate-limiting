# Implementing a distributed rate limiter in rust

This repository proposes an implementation of a distributed rate limiter written
in Rust, and a REST API project example using the same rate limiter library
to limit the number of incoming requests based on the caller's IP address.

The implemented solution is a slightly revised, simplified, implementation of the
[Token bucket](https://en.wikipedia.org/wiki/Token_bucket) algorithm
and relies on [Redis](https://redis.io/) for its distributed state management.

This repository includes 2 main directories:

- [rate-limiter-rs](./rate-limiter-rs/): the rate limiter library;
- [carbon-intensity-api](./carbon-intensity-api/): a sample project exposing
a REST API, that uses the above mentioned rate limiter component.
