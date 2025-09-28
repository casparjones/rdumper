# Multi-stage Dockerfile for rDumper
# Stage 1: Frontend build
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# Stage 2: Rust backend build
FROM rust:alpine AS backend-builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    sqlite-dev \
    sqlite-static

WORKDIR /app
COPY backend/Cargo.toml ./
COPY backend/src ./src

# Build the application (Alpine uses musl by default)
RUN cargo build --release

# Stage 3: Build mydumper + myloader
FROM alpine:latest AS mydumper-builder

RUN apk add --no-cache \
    build-base \
    cmake \
    glib-dev \
    pcre-dev \
    mariadb-connector-c-dev \
    zlib-dev \
    wget \
    ca-certificates

WORKDIR /src
ENV MYDUMPER_VERSION=v0.20.1-2
RUN wget https://github.com/mydumper/mydumper/archive/refs/tags/${MYDUMPER_VERSION}.tar.gz -O mydumper.tar.gz \
    && tar xzf mydumper.tar.gz --strip 1


RUN mkdir build && cd build && \
    cmake .. -DCMAKE_BUILD_TYPE=Release && \
    make && make install

# Stage 4: Runtime
FROM alpine:latest

# Install runtime dependencies (for rdumper and mydumper)
RUN apk add --no-cache \
    ca-certificates \
    wget \
    sqlite \
    glib \
    pcre \
    mariadb-connector-c

# Create app user and directories with proper permissions
RUN adduser -D -s /bin/false rdumper && \
    mkdir -p /data/backups /data/logs /app/static && \
    chown -R rdumper:rdumper /data /app && \
    chmod 755 /data /data/backups /data/logs

WORKDIR /app

# Copy built artifacts
COPY --from=backend-builder /app/target/release/rdumper-backend ./rdumper-backend
COPY --from=frontend-builder /app/frontend/dist ./static
COPY --from=mydumper-builder /usr/local/bin/mydumper /usr/local/bin/
COPY --from=mydumper-builder /usr/local/bin/myloader /usr/local/bin/

# Change ownership
RUN chown -R rdumper:rdumper /app

# Switch to non-root user
USER rdumper

# Expose port
EXPOSE 3000

# Volumes für Config DB und Backups
VOLUME ["/app/data"]

# Environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///data/rdumper.db
ENV BACKUP_DIR=/data/backups
ENV LOG_DIR=/data/logs
ENV STATIC_DIR=/app/static

# Health check (auskommentiert, weil /api/system 404 liefert)
# HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
#     CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

# Start the application
CMD ["./rdumper-backend", "--host", "0.0.0.0", "--port", "3000", "--database-url", "sqlite:///data/rdumper.db", "--backup-dir", "/data/backups", "--log-dir", "/data/logs", "--static-dir", "/app/static"]
