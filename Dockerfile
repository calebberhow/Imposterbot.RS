########################################################################################################################
# Toolchain setup stage
########################################################################################################################

FROM lukemathwalker/cargo-chef:latest-rust-1 AS toolchain
RUN rustup target add x86_64-unknown-linux-musl && \
    apt update && \
    apt install -y musl-tools musl-dev && \
    update-ca-certificates

# Install OpenSSL development libraries and pkg-config
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Install sqlx
RUN cargo install sqlx-cli

########################################################################################################################
# Cargo chef prepare
########################################################################################################################
FROM toolchain AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

########################################################################################################################
# Imposterbot build stage
########################################################################################################################

FROM toolchain AS builder
ARG DATABASE_URL=sqlite:/imposterbot_data.db

# Build and cache dependencies separately from app build
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --target x86_64-unknown-linux-musl --release --recipe-path recipe.json

# Copy source
COPY . .

# Setup temp database (required for build)
RUN cargo sqlx database setup

# Configure app user
RUN groupadd --gid 10001 appgroup && \
    useradd --uid 10001 --gid appgroup --shell /bin/bash --create-home appuser

# Build app
RUN cargo build --target x86_64-unknown-linux-musl --release

########################################################################################################################
# Imposterbot image
########################################################################################################################

FROM ubuntu:24.04

# Setup default environment variables
ENV DATABASE_URL=sqlite:/data/imposterbot_data.db
ENV DATA_DIRECTORY=/data
ENV MEDIA_DIRECTORY=/media

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

COPY --from=builder --chown=appuser:appgroup ./target/x86_64-unknown-linux-musl/release/imposterbot /app/imposterbot
COPY ./migrations /migrations
COPY ./media /media

# TODO: Setting the user prevents write access to /data volume
# USER appuser

ENTRYPOINT ["/app/imposterbot"]