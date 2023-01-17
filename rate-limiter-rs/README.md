# rate-limiter-rs

A distributed rate limiter component written in Rust, that follows a slightly
revised and simplified version of the [Token bucket algorithm](https://en.wikipedia.org/wiki/Token_bucket).

## Implementation details

Unlike traditional token bucket rate limiters, this implementation does not
refill the bucket at a fixed interval rate, but it creates a bucket on the
very first request belonging to the same ip address, or a custom origin
identifier, with a configured expiry time.

Compared to a classic token bucket implementation
such approach should be slightly more efficient with regards to memory
consumption, as we have buckets held in memory just for those IP addresses
(or origin identifiers) that actually hit our service.

The disadvantage of this solution compared to a canonical token bucket
rate limiter is that once the request budget is reached, the caller
should wait the bucket expiration before firing any new request, as
opposed to having the request budget bumped of one (or more) token
at a regular interval, typically 1 or few seconds.

The algorithm is defined [here](./src/lib.rs#L54).

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

- [ ] More flexibility, right now the implementation is coupled to Redis
- [ ] Leverage the use of feature flags to selectively include specific
rate limiter implementations
- [ ] Redis: Add support for async
- [ ] Redis: Add support for TLS
- [ ] Improved local testing: Ideally it should be possible to mock redis,
responses if needed
