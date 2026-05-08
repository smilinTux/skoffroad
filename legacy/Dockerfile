# Build stage
FROM rust:latest as builder

WORKDIR /usr/src/sandk-offroad

# Copy only necessary files for building
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets

# Install build dependencies and build the project
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    libudev-dev \
    libwayland-dev \
    libx11-dev \
    libxkbcommon-dev \
    && rm -rf /var/lib/apt/lists/* && \
    # Build with debug symbols for better error messages
    RUSTFLAGS="-C debuginfo=2" cargo build --release

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl1.1 \
    libasound2 \
    libudev1 \
    libwayland-client0 \
    libx11-6 \
    libxkbcommon0 \
    libglib2.0-0 \
    libvulkan1 \
    mesa-vulkan-drivers \
    libxcb1 \
    libxcb-render0 \
    libxcb-shape0 \
    libxcb-xfixes0 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary, libraries and assets from builder stage
COPY --from=builder /usr/src/sandk-offroad/target/release/sandk-offroad /app/
COPY --from=builder /usr/src/sandk-offroad/target/release/deps/*.so /app/
COPY --from=builder /usr/src/sandk-offroad/assets /app/assets

# Set library path and other environment variables
ENV LD_LIBRARY_PATH=/app:/usr/local/lib:/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH
ENV RUST_BACKTRACE=1

# Set the entrypoint
ENTRYPOINT ["/app/sandk-offroad"] 