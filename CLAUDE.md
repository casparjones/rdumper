Du bist ein erfahrener Fullstack-Entwickler. Bitte implementiere ein Projekt mit folgendem Setup:

## Projektname
**DumpTrack** (Rust GUI Wrapper für mydumper/myloader)

## Technologie-Stack
- **Frontend**: Vue 3, Tailwind CSS v4, DaisyUI v5
- **Backend**: Rust (axum oder actix-web), Ansteuerung von `mydumper` & `myloader` als Wrapper
- **Containerisierung**: Dockerfile
- **CI/CD**: GitHub Actions Workflow zum automatischen Bauen & Pushen des Images

## Hauptfunktionen
1. **Dashboard** mit Status der letzten Aktivitäten
2. **Multi-DB Configs**: Datenbanken hinzufügen, bearbeiten, löschen (Host, Port, User, PW, DB-Name)
3. **Tasks** (Backup-Pläne):
    - tägliches Backup kompletter DBs
    - Optionen: Kompression (gzip, zstd), Zeitplan (cron-ähnlich)
4. **Clean-up**: Backups automatisch löschen (z. B. älter als 30 Tage)
5. **Jobs**:
    - Übersicht laufender Jobs mit Prozentfortschritt
    - Logging (Start, Dauer, Error)
6. **Task-Übersicht**:
    - geplanter nächster Start
    - letzter Status & Laufzeit
7. **Backup-Verwaltung**:
    - Liste vorhandener Backups
    - Restore-Optionen:
        - gleiche DB überschreiben
        - neue DB erstellen
    - Restore läuft als Task/Job
8. **Systeminfo**:
    - Version von `mydumper` & `myloader`
    - Systeminformationen (Linux-Distro, Kernel)
    - Version der GUI (aus `Cargo.toml`/Git-Tag)
9. **Deployment**:
    - Dockerfile zum Container-Bau
    - GitHub Action zum automatischen Bauen und Pushen von Docker-Images

---

## Detaillierte To-Do-Liste (Reihenfolge)

### 1. Projekt-Setup
- Rust Backend mit axum oder actix-web starten
- API-Struktur definieren (`/api/databases`, `/api/tasks`, `/api/jobs`, `/api/backups`, `/api/system`)
- SQLite oder Postgres als Storage für Configs/Tasks nutzen
- Vue + Vite + Tailwind 4 + DaisyUI 5 Frontend initialisieren
- Basis-Dockerfile für Rust + Node + mydumper erstellen

### 2. Datenbank-Config Management
- Backend: CRUD-Endpunkte für Datenbankconfigs
- Frontend: UI für Eingabe + Liste aller Configs

### 3. Task-System
- Rust: Scheduler (cronähnlich, z. B. `tokio_cron_scheduler`)
- API: CRUD für Tasks
- Task enthält:
    - DB-Config-ID
    - Zeitpunkt / Intervall
    - Optionen (Kompression, Cleanup-Regeln)

### 4. Backup-Execution
- Rust: Wrapper für `mydumper` → Prozess starten, stdout/stderr abfangen
- Fortschritt (z. B. per Log-Zeilen von mydumper) parsen
- Backups in definiertes Verzeichnis schreiben (z. B. `/data/backups/<db>/<timestamp>`)

### 5. Clean-up System
- Rust: periodischer Job, löscht Backups > N Tage
- Konfigurierbar pro Task

### 6. Jobs & Logs
- Datenbankmodell für Jobs (Startzeit, Endzeit, Dauer, Status, Log-Ausgabe)
- API-Endpunkte `/api/jobs`
- Frontend: Liste mit Filter & Detailansicht

### 7. Backup-Übersicht & Restore
- Backend:
    - Backup-Liste auslesen
    - Restore-Job starten (`myloader` Wrapper)
    - Optionen: gleiche DB überschreiben oder neue DB erstellen
- Frontend: Tabelle mit Buttons für "Restore" + Optionen

### 8. Dashboard
- Aggregierte Infos:
    - Anzahl DBs
    - Anzahl Tasks
    - Letzte Backups
    - Nächste geplante Jobs
- Fortschrittsbalken für laufende Jobs

### 9. Systeminfo
- Endpunkt `/api/system`
- Holt Version von `mydumper`, `myloader`, OS & Kernel
- GUI-Version aus `Cargo.toml` oder Git-Commit

### 10. Deployment
- Multi-Stage Dockerfile:
    - Stage 1: Rust + Node.js für Build
    - Stage 2: schlankes Image mit Backend, mydumper/myloader, kompiliertem Frontend
- GitHub Action:
    - Build & Push Docker-Image (Docker Hub oder GitHub Container Registry)

---

## Hinweise für Claude
- Immer mit Vue 3 + Tailwind CSS v4 + DaisyUI v5 arbeiten, keine alten Libraries.
- Backend strikt in Rust, mydumper/myloader nur als Wrapper (kein Re-Implementieren).
- Sauberes Error-Handling bei Job-Logs.
- Möglichst modulare Struktur, damit später weitere Features ergänzt werden können.
