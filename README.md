# Rate limiting

This project is meant to explain how to possibly implement a distributed rate limiter component to limit incoming requests to your API.
This repository includes 2 main directories: 
- [rate-limiter-rs](./rate-limiter-rs/): the actual rate limiter library
- [carbon-intensity-api](./carbon-intensity-api/): a sample project exposing a REST API, that uses the above mentioned rate limiter library.