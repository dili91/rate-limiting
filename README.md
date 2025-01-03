# Implementing a distributed rate limiter in rust

This repository proposes an implementation of a distributed rate limiter written
in Rust, and a REST API project example using the same rate limiter library
to limit the number of incoming requests based on the caller's IP address.

It is a demo project that helped me writing [this blog post](https://adilisio.com/posts/implementing-a-rate-limiter-for-our-api-in-rust/).

There are 2 main directories:

- [rate-limiter-rs](./rate-limiter-rs/): the rate limiter library, that offers
both a _fixed window_ and a _sliding window_ implementations;
- [carbon-intensity-api](./carbon-intensity-api/): a sample project exposing
a REST API, that uses the above mentioned rate limiter component.
