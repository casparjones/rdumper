#!/bin/bash

# Build script for rDumper Docker image with build-time information

set -e

# Get build-time information
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RUSTC_VERSION=$(rustc --version 2>/dev/null || echo "unknown")

echo "Building rDumper Docker image..."
echo "Git Commit: $GIT_COMMIT"
echo "Build Date: $BUILD_DATE"
echo "Rust Version: $RUSTC_VERSION"

# Build the Docker image with build arguments
docker build \
    --build-arg GIT_COMMIT="$GIT_COMMIT" \
    --build-arg BUILD_DATE="$BUILD_DATE" \
    --build-arg RUSTC_VERSION="$RUSTC_VERSION" \
    -t rdumper:latest \
    .

echo "Build completed successfully!"
echo "To run the container:"
echo "docker run -p 3000:3000 rdumper:latest"
