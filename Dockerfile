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

# Stage 3: Runtime
FROM alpine:latest

# Install dependencies (mydumper will be added separately)
RUN apk add --no-cache \
    ca-certificates \
    wget \
    sqlite \
    sqlite-dev

# Create app user and directories with proper permissions
RUN adduser -D -s /bin/false rdumper && \
    mkdir -p /data/backups /data/logs /app/static && \
    chown -R rdumper:rdumper /data /app && \
    chmod 755 /data /data/backups /data/logs

WORKDIR /app

# Copy built artifacts
COPY --from=backend-builder /app/target/release/rdumper-backend ./rdumper-backend
COPY --from=frontend-builder /app/frontend/dist ./static

# Change ownership
RUN chown -R rdumper:rdumper /app

# Switch to non-root user
USER rdumper

# Expose port
EXPOSE 3000

# Volumes für Config DB und Backups
VOLUME ["/app/backend/data"]

# Environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///data/rdumper.db
ENV BACKUP_DIR=/data/backups
ENV LOG_DIR=/data/logs
ENV STATIC_DIR=/app/static

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/api/system || exit 1

# Container im Leerlauf halten
CMD ["tail", "-f", "/dev/null"]

# Start the application
# CMD ["./rdumper-backend", "--host", "0.0.0.0", "--port", "3000", "--database-url", "sqlite:///data/rdumper.db", "--backup-dir", "/data/backups", "--log-dir", "/data/logs", "--static-dir", "/app/static"]