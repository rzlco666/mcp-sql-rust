# syntax=docker/dockerfile:1

FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/mcp-sql-rust /usr/local/bin/mcp-sql-rust
ENTRYPOINT ["/usr/local/bin/mcp-sql-rust"]
