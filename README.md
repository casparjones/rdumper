# rDumper

A modern web-based GUI wrapper for `mydumper` and `myloader` built with Rust and Vue.js.

## Features

- 🗄️ **Database Management**: Configure and manage multiple MySQL database connections
- ⏰ **Scheduled Backups**: Create and manage backup tasks with cron-like scheduling
- 📊 **Job Monitoring**: Real-time monitoring of backup and restore operations
- 💾 **Backup Management**: Browse, restore, and manage your database backups
- 🧹 **Automatic Cleanup**: Configurable automatic cleanup of old backups
- 🎨 **Modern UI**: Clean, responsive interface built with Vue 3, Tailwind CSS, and DaisyUI
- 🐳 **Docker Ready**: Multi-stage Docker build for easy deployment
- 🔒 **Secure**: Non-root container execution with proper permission handling

## Technology Stack

### Backend (Rust)
- **Framework**: Axum (async web framework)
- **Database**: SQLite with SQLx (compile-time checked queries)
- **Scheduling**: tokio-cron-scheduler
- **CLI**: clap for command-line arguments
- **Logging**: tracing + tracing-subscriber

### Frontend (Vue 3)
- **Framework**: Vue 3 with Composition API
- **Build Tool**: Vite
- **Styling**: Tailwind CSS v4 + DaisyUI v5
- **Routing**: Vue Router
- **HTTP Client**: Axios

## Prerequisites

- Docker and Docker Compose (recommended)
- OR manually:
  - Rust 1.70+
  - Node.js 20+
  - mydumper/myloader installed

## Quick Start with Docker

1. Clone the repository:
```bash
git clone <repository-url>
cd rdumper
```

2. Start with Docker Compose:
```bash
docker-compose up -d
```

3. Access the application:
- Web UI: http://localhost:3000
- API: http://localhost:3000/api

## Manual Development Setup

### Backend Development

```bash
cd backend
cargo run
```

The backend will start on http://localhost:3000

### Frontend Development

```bash
cd frontend
npm install
npm run dev
```

The frontend dev server will start on http://localhost:5173 with proxy to backend.

## Building for Production

### Build Frontend
```bash
cd frontend
npm run build
```

### Build Backend
```bash
cd backend
cargo build --release
```

### Build Docker Image
```bash
docker build -t rdumper .
```

## Configuration

### Environment Variables

- `RUST_LOG`: Logging level (default: `info`)
- `DATABASE_URL`: SQLite database path (default: `sqlite:rdumper.db`)
- `BACKUP_DIR`: Backup storage directory (default: `/data/backups`)
- `STATIC_DIR`: Frontend static files directory (default: `../frontend/dist`)

### Command Line Arguments

```bash
./rdumper-backend --help
```

- `--host`: Server host (default: `0.0.0.0`)
- `--port`: Server port (default: `3000`)
- `--database-url`: SQLite database URL
- `--backup-dir`: Backup storage directory
- `--static-dir`: Static files directory

## API Documentation

### Endpoints

- `GET /api/database-configs` - List database configurations
- `POST /api/database-configs` - Create database configuration
- `GET /api/tasks` - List backup tasks
- `POST /api/tasks` - Create backup task
- `GET /api/jobs` - List jobs
- `GET /api/backups` - List backups
- `GET /api/system` - System information

## Usage

### 1. Configure Databases
Add your MySQL database connections through the "Databases" section.

### 2. Create Backup Tasks
Set up scheduled backup tasks with:
- Cron schedule expression
- Compression type (none, gzip, zstd)
- Automatic cleanup settings

### 3. Monitor Jobs
Track backup and restore operations in real-time through the "Jobs" section.

### 4. Manage Backups
Browse and restore backups through the "Backups" section.

## Docker Deployment

### Using Docker Compose (Recommended)

```yaml
version: '3.8'
services:
  rdumper:
    image: ghcr.io/your-username/rdumper:latest
    ports:
      - "3000:3000"
    volumes:
      - rdumper_data:/data
      - ./backups:/data/backups
    environment:
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  rdumper_data:
```

### Using Docker Run

```bash
docker run -d \
  --name rdumper \
  -p 3000:3000 \
  -v rdumper_data:/data \
  -v $(pwd)/backups:/data/backups \
  -e RUST_LOG=info \
  rdumper:latest
```

## Security Considerations

- The application runs as a non-root user in the container
- Database passwords are stored encrypted
- All file operations are performed within the designated backup directory
- Container includes only necessary dependencies

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

[Add your license here]

## Support

[Add support information here]
