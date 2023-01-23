# rate-limiter-rs

A rate limiter library written in Rust and based on Redis that offers
both a _fixed window_ and a _sliding window_ implementations.

## Implementation details

Detailed information about how the 2 algorithms are implemented are
available as documentation comments.

## Requirements

As of today, Redis is a strict requirement of this library.

## Building

```shell
cargo build
```

## Testing

```shell
cargo nextest run
```

&ast; Please note that some of the tests currently require a running Redis
instance on your local machine. To quickly start a redis server locally, you
can spy on the [CI workflow](../.github/workflows/rate_limiter_rs.yml).

## Areas of improvements

- [ ] Leverage the use of feature flags to selectively include specific
rate limiter implementations ?
- [ ] Redis: Add support for async
- [ ] Redis: Add support for TLS
- [ ] Improved local testing: Ideally it should be possible to mock redis,
responses if needed
