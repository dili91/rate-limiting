FROM rust:1.66 as builder
COPY . .
WORKDIR /usr/workspace
COPY . .

RUN cargo install --path /usr/workspace/carbon-intensity-api

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/carbon-intensity-api /usr/local/bin/carbon-intensity-api

EXPOSE 9000

CMD ["carbon-intensity-api"]