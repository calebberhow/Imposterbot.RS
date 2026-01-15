########################################################################################################################
# Toolchain setup stage
########################################################################################################################

FROM lukemathwalker/cargo-chef:latest-rust-1 AS toolchain

# CMake and C compiler are required to build openssl from C source code for songbird
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

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
COPY migration migration
RUN cargo chef cook --release --recipe-path recipe.json --features "voice"

# Copy source
COPY . .

# Configure app user
RUN groupadd --gid 10001 appgroup && \
    useradd --uid 10001 --gid appgroup --shell /bin/bash --create-home appuser

# Build app
RUN cargo build --release --features "voice"

########################################################################################################################
# Imposterbot image
########################################################################################################################

FROM ubuntu:24.04

# Setup default environment variables
ENV DATABASE_URL=sqlite:/data/imposterbot_data.db?mode=rwc
ENV DATA_DIRECTORY=/data
ENV MEDIA_DIRECTORY=/media

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

COPY --from=builder --chown=appuser:appgroup ./target/release/imposterbot /app/imposterbot
COPY ./media /media

COPY ./deps/yt-dlp /usr/local/bin/yt-dlp
RUN chmod u+rx /usr/local/bin/yt-dlp

## App is dynamically linked to libopus
# yt-dlp requires python3 executable during runtime.
RUN apt-get update && \
    apt-get install -y libopus0 python3 && \
    rm -rf /var/lib/apt/lists/*

# TODO: Setting the user prevents write access to /data volume
# USER appuser

ENTRYPOINT ["/app/imposterbot"]