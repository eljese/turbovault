# syntax=docker/dockerfile:1.9
FROM rust:latest as builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# Build with minimal features for STDIO performance
RUN cargo build --release --package turbovault

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/turbovault /usr/local/bin/

# Match NFS permissions
RUN groupadd -g 3000 obsidian && \
    useradd -m -u 3000 -g 3000 obsidian

RUN mkdir -p /var/obsidian-vault && chown obsidian:obsidian /var/obsidian-vault
USER obsidian
WORKDIR /var/obsidian-vault

ENV RUST_LOG=info
ENV OBSIDIAN_VAULT_PATH=/var/obsidian-vault

# Default to STDIO transport
ENTRYPOINT ["/usr/local/bin/turbovault", "--profile", "production", "--init", "--transport", "stdio"]