# Multi-stage build for LogNarrator Client

# Build Rust components
FROM rust:1.70-slim as rust-builder
WORKDIR /app

# Create empty project to cache dependencies
RUN USER=root cargo new --bin rust
WORKDIR /app/rust
COPY ./rust/Cargo.toml ./Cargo.toml
RUN cargo build --release && rm -rf src/

# Build actual project
COPY ./rust/src ./src
RUN cargo build --release

# Build Go components
FROM golang:1.20-alpine as go-builder
WORKDIR /app

COPY ./go/go.mod ./go/go.sum ./
RUN go mod download

COPY ./go ./
RUN CGO_ENABLED=0 GOOS=linux go build -o /collector ./cmd/collector

# Final image
FROM debian:bullseye-slim
WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsodium23 \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create directory structure
RUN mkdir -p /app/bin /app/config /app/data

# Copy binaries from build stages
COPY --from=rust-builder /app/rust/target/release/mcp_client /app/bin/
COPY --from=go-builder /collector /app/bin/

# Copy config files
COPY ./config /app/config/

# Create volume mount points
VOLUME ["/app/data", "/app/config"]

# Set environment variables
ENV PATH="/app/bin:${PATH}"

CMD ["/app/bin/collector"]
