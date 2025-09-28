# Frontend Environment Configuration

## Environment Variables

Das Frontend unterstützt folgende Environment-Variablen für die API-Konfiguration:

### API Configuration

- **`VITE_API_URL`** (optional): Vollständige API-URL
  - Beispiel: `http://localhost:3000` oder `https://your-domain.com:3000`
  - Wenn nicht gesetzt, wird automatisch die aktuelle Domain erkannt

- **`VITE_API_PORT`** (optional): API Port
  - Standard: `3000`
  - Wird nur verwendet wenn `VITE_API_URL` nicht gesetzt ist

### Development Server

- **`VITE_DEV_PORT`** (optional): Development Server Port
  - Standard: `5173`

## Verwendung

### Development
```bash
# Standard (localhost:3000)
npm run dev

# Mit custom API Port
VITE_API_PORT=8080 npm run dev

# Mit vollständiger API URL
VITE_API_URL=http://localhost:8080 npm run dev
```

### Production
```bash
# Mit automatischer Domain-Erkennung
VITE_API_PORT=3000 npm run build

# Mit expliziter Domain
VITE_API_URL=https://your-domain.com:3000 npm run build
```

## Automatische Domain-Erkennung

Das Frontend erkennt automatisch:
- **Development**: `localhost` oder `127.0.0.1` mit konfigurierbarem Port
- **Production**: Gleiche Domain wie die Frontend-App mit konfigurierbarem Port

Die Erkennung funktioniert sowohl für HTTP als auch HTTPS.
