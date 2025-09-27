# rDumper

A modern **web-based GUI wrapper** for [`mydumper`](https://github.com/mydumper/mydumper) and [`myloader`](https://github.com/mydumper/mydumper) — built with **Rust** (backend) and **Vue 3 + TailwindCSS + DaisyUI** (frontend).

---

## ✨ Features

- 🗄️ **Database Management**: Configure and manage multiple MySQL database connections  
- ⏰ **Scheduled Backups**: Create recurring tasks with cron-like scheduling  
- 📊 **Job Monitoring**: Real-time tracking of backup and restore operations  
- 💾 **Backup Management**: Browse, restore, and manage your database backups  
- 🧹 **Automatic Cleanup**: Remove old backups after a configurable retention period  
- 🎨 **Modern UI**: Responsive interface built with Vue 3, TailwindCSS v4, and DaisyUI v5  
- 🐳 **Docker Ready**: Multi-stage Docker build for easy deployment  
- 🔒 **Secure**: Non-root container execution with proper permission handling  

---

## 🛠️ Technology Stack

### Backend (Rust)
- **Framework**: [Axum](https://github.com/tokio-rs/axum) (async web framework)  
- **Database**: SQLite with [SQLx](https://github.com/launchbadge/sqlx) (compile-time checked queries)  
- **Scheduling**: [tokio-cron-scheduler](https://github.com/emabee/tokio-cron-scheduler)  
- **CLI**: [clap](https://github.com/clap-rs/clap) for argument parsing  
- **Logging**: [tracing](https://github.com/tokio-rs/tracing)  

### Frontend (Vue 3)
- **Framework**: Vue 3 (Composition API)  
- **Build Tool**: Vite  
- **Styling**: TailwindCSS v4 + DaisyUI v5  
- **Routing**: Vue Router  
- **HTTP Client**: Axios  

---

## 🚀 Quick Start (Docker)

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd rdumper

2. Start with Docker Compose:

   ```bash
   docker-compose up -d
   ```

3. Access the application:

    * Web UI → [http://localhost:3000](http://localhost:3000)
    * API → [http://localhost:3000/api](http://localhost:3000/api)

---

## 💻 Manual Development Setup

### Backend

```bash
cd backend
cargo run
```

Backend runs on [http://localhost:3000](http://localhost:3000).

### Frontend

```bash
cd frontend
npm install
npm run dev
```

Frontend runs on [http://localhost:5173](http://localhost:5173) with proxy to backend.

---

## ⚙️ Configuration

### Environment Variables

* `RUST_LOG`: Logging level (default: `info`)
* `DATABASE_URL`: SQLite database path (default: `sqlite:rdumper.db`)
* `BACKUP_DIR`: Backup storage directory (default: `/data/backups`)
* `STATIC_DIR`: Frontend static files directory (default: `../frontend/dist`)

### Command Line Arguments

```bash
./rdumper-backend --help
```

---

## 📖 API Endpoints

* `GET /api/database-configs` → List database configs
* `POST /api/database-configs` → Create database config
* `GET /api/tasks` → List backup tasks
* `POST /api/tasks` → Create backup task
* `GET /api/jobs` → List jobs
* `GET /api/backups` → List backups
* `GET /api/system` → System information

---

## 🐳 Docker Deployment

### Docker Compose (Recommended)

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

### Docker Run

```bash
docker run -d \
  --name rdumper \
  -p 3000:3000 \
  -v rdumper_data:/data \
  -v $(pwd)/backups:/data/backups \
  -e RUST_LOG=info \
  rdumper:latest
```

---

## 🔒 Security Notes

* Runs as a non-root user in the container
* Database credentials stored encrypted
* All file operations restricted to backup directory
* Container includes only required dependencies

---

## 👥 Authors

- **Frank** - Project Owner & Lead Developer
- **Claude (Anthropic)** - AI Assistant & Code Contributor
- **ChatGPT (OpenAI)** - AI Assistant & Code Contributor

*“Give credit where credit is due.”*  
*“Honor should be given to whom honor is due.”* 😊

---

## 🤝 Contributing

1. Fork this repository
2. Create a feature branch
3. Commit and push your changes
4. Open a Pull Request

---

## 📄 License

MIT License

---

## 🛠️ Support

If you encounter any bugs or have feature requests, please open an [issue on GitHub](https://github.com/casparjones/rdumper/issues).
Your feedback helps improve **rDumper** for everyone. 🚀
