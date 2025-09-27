# rDumper Test Data Setup

Dieses Verzeichnis enthält Scripts und Konfigurationen für die Einrichtung von Testdaten für rDumper.

## 🚀 Schnellstart

Nach einem Reset der Datenbank können Sie die Testdaten mit folgendem Befehl einrichten:

```bash
cd backend
./setup_test_data.sh
```

## 📋 Testdaten

Das Script erstellt folgende Testkonfigurationen:

### Datenbank-Konfigurationen

1. **Test Database**
   - Host: `127.0.0.1:3306`
   - Benutzer: `root`
   - Passwort: (leer)
   - Datenbank: `test_db`

2. **Production Database** (Beispiel)
   - Host: `mysql.example.com:3306`
   - Benutzer: `backup_user`
   - Passwort: `secure_password`
   - Datenbank: `production_db`

### Tasks

1. **Daily Test Backup**
   - Aktiviert: ✅ Ja
   - Zeitplan: `0 2 * * *` (täglich um 2:00 Uhr)
   - Komprimierung: `gzip`
   - Cleanup: 7 Tage

2. **Hourly Production Backup**
   - Aktiviert: ❌ Nein (deaktiviert)
   - Zeitplan: `0 * * * *` (stündlich)
   - Komprimierung: `gzip`
   - Cleanup: 30 Tage

## 🔧 Manuelle Einrichtung

Falls Sie die Testdaten manuell einrichten möchten:

```bash
# Datenbank leeren
rm -f data/db/rdumper.db

# Backend starten (erstellt Tabellen)
timeout 5s cargo run || true

# Testdaten einfügen
sqlite3 data/db/rdumper.db < setup_test_data.sql
```

## 📁 Dateien

- `setup_test_data.sh` - Automatisches Setup-Script
- `setup_test_data.sql` - SQL-Script mit Testdaten

## 🌐 Verwendung

Nach dem Setup können Sie:

1. Das Backend starten: `cargo run`
2. Das Frontend öffnen: `http://localhost:3000`
3. Die Testkonfigurationen im Frontend verwenden

## ⚙️ Konfiguration

Die Pfade können über Umgebungsvariablen konfiguriert werden:

- `BACKUP_DIR` - Backup-Verzeichnis (Standard: `./data/backups`)
- `LOG_DIR` - Log-Verzeichnis (Standard: `./data/logs`)
- `DATABASE_URL` - Datenbank-URL (Standard: `sqlite://data/db/rdumper.db`)

**Hinweis:** Relative Pfade (ohne `/`) sind relativ zum Backend-Verzeichnis. Absolute Pfade (mit `/`) werden für Docker-Container verwendet.
