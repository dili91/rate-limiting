FROM rust:1.82 AS builder
WORKDIR /usr/workspace
COPY . .

RUN cargo install --path /usr/workspace/carbon-intensity-api

FROM debian:bookworm-20241223
COPY --from=builder /usr/local/cargo/bin/carbon-intensity-api /usr/local/bin/carbon-intensity-api

EXPOSE 9000

CMD ["carbon-intensity-api"]