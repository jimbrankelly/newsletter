# We use the latest Rust stable release as base image
#FROM rust:1.94.0 AS builder
FROM lukemathwalker/cargo-chef:latest-rust-1.94.0 AS chef
WORKDIR /app
# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y
# Copy all files from our working environment to our Docker image 
FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached. 
COPY . .
ENV SQLX_OFFLINE=true
# Let's build our binary!
# We'll use the release profile to make it faaaast
RUN cargo build --release

# Runtime stage
#FROM rust:1.94.0-slim AS runtime
#WORKDIR /app

FROM debian:trixie-slim AS runtime
WORKDIR /app
# Install OpenSSL - it is dynamically linked by some of our dependencies
# Install ca-certificates - it is needed to verify TLS certificates
# when establishing HTTPS connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
# Copy the compiled binary from the builder environment 
# to our runtime environment
COPY --from=builder /app/target/release/newsletter newsletter
# We need the configuration file at runtime!
COPY configuration configuration
# When `docker run` is executed, launch the binary!
ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./newsletter"]
