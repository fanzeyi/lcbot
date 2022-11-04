FROM rust:1.65 as builder

COPY . .

RUN cargo build --release -j1

FROM debian:buster-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates
COPY .env /app/.env
COPY --from=builder ./target/release/lcbot /usr/local/lcbot
CMD ["/usr/local/lcbot"]
