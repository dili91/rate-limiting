# carbon-intensity-api

A sample API project built on [Actix](https://actix.rs/) to showcase the use
of the [rate-limiter-rs](../rate-limiter-rs/) library.

## Implementation details

The current codebase comes with 2 routes:

- a [`GET /health_check`](./src/routes/healt_check.rs) endpoint that simply
returns an _I'm alive_ message. This endpoint is of course not subject to
rate limiting;
- a [`GET /carbon/intensity`](./src/routes/intensity/get_intensity.rs)
endpoint that returns (as of now) some mocked data on whether the energy
you're using can be considered clean or not. This endpoint is rate-limited,
as per below configuration.

## Rate limiting configuration

Currently the rate limiter is configured to have a window size of `5`,
and a duration of `60s`. Rate limiting is based on the IP address from which the
request originates. The same IP address can invoke the carbon intensity endpoint
only 5 times in a minute. Every further request should be throttled and the caller
should receive [a standard 429 HTTP status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/429)
and a `retry-after` HTTP response header, including the information on when to expect
a positive response, in seconds

## Samples

### Non rate-limited endpoint

The `health_check` endpoint ideally _always_ return a simple 200 response:

```shell
http :9000/health_check
<~
HTTP/1.1 200 OK
content-length: 19
date: Sun, 15 Jan 2023 17:26:51 GMT

I'm up and running.
```

### Rate limited endpoint, success

When calling a rate limited endpoint, in case the request is allowed, we
should get as well a 200 and a custom `x-remaining-request` HTTP header
that includes the updated counter of the available requests in
the current window:

```shell
http :9000/carbon/intensity
<~
HTTP/1.1 200 OK
content-length: 114
content-type: application/json
date: Sun, 15 Jan 2023 17:28:50 GMT
x-remaining-request: 3

{
    "from": "2018-01-20T12:00Z",
    "intensity": {
        "actual": 263,
        "forecast": 266,
        "index": "Moderate"
    },
    "to": "2018-01-20T12:30Z"
}
```

### Rate limited endpoint, throttled request

If our request was throttled the following output is expected:

```shell
http :9000/carbon/intensity
<~
HTTP/1.1 429 Too Many Requests
content-length: 22
date: Sun, 15 Jan 2023 17:32:23 GMT
retry-after: 56

You've been throttled!
```

## Running

To run the app:

```shell
just local-run
```

> [!NOTE]  
> A local Redis instance is required!

## Testing

It's possible to test the API in different ways. Each of them requires a
local Redis instance running.

### Manual tests

Once running, you can use curl/httpie to manually test the app. See the
above [samples](#samples) section.

### Automated E2E tests

To run the automated e2e test suite:

```shell
just test
```

### Distributed/Load tests

To run the tests in some more real-looking environment, I've configured
a more complex [docker-compose](compose.yaml) setup, made of:

- 1 Nginx instance acting as reverse proxy and load balancing
entrypoint, listening on port 8080;
- 3 replicas of the carbon-intensity-api app;
- 1 Redis instance.

To boot the stack you can simply run the boot script:

```shell
just compose-run
```

This will docker-compose up the above stack in detached mode.

You can then either again manually invoking the API manually, this time
through the reverse proxy / load balancer listening on port `8080` or
simulate a very basic, local load test with the help the [k6](https://k6.io/)
software, as described on this [test scenario](./distributed_test.js).

You can run the distributed test with the k6 CLI, either via docker or
installed locally:

```shell
just load-test
```

The test scenario will fire 100 requests on 25 simultaneous virtual users
/ threads and verify that out of 100 requests, 5 were successful and 95
throttled.
