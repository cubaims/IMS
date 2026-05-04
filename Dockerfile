FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p cuba-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/cuba-api /usr/local/bin/cuba-api
EXPOSE 8080
CMD ["cuba-api"]
