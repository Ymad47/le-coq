# Le Coq — Deployment Guide

**Backup, Monitoring, CI/CD — Phase 1 à Phase 3**

---

## 🗄️ Backup PostgreSQL

### Phase 1 (pg_dump + cron)

**Script de backup :**
```bash
#!/bin/bash
# /home/yaya/scripts/backup-lecoq.sh

set -euo pipefail

DATE=$(date +%Y-%m-%d_%H-%M-%S)
BACKUP_DIR="/backup/lecoq"
DB_NAME="lecoq"
DB_USER="lecoq"
DB_HOST="localhost"
RETENTION_DAYS=7
STORAGE_BOX_USER="u123456"
STORAGE_BOX_HOST="u123456.your-storagebox.de"

# Create backup dir
mkdir -p "$BACKUP_DIR"

# Dump (custom format, compressed)
echo "[$(date -Iseconds)] Starting backup..."
pg_dump \
    -h "$DB_HOST" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    -F c \
    -Z 5 \
    -f "$BACKUP_DIR/lecoq_$DATE.dump"

# Verify backup
if pg_restore --list "$BACKUP_DIR/lecoq_$Date.dump" > /dev/null 2>&1; then
    echo "[$(date -Iseconds)] Backup verified successfully"
else
    echo "[$(date -Iseconds)] ERROR: Backup verification failed"
    exit 1
fi

# Upload to Hetzner Storage Box
echo "[$(date -Iseconds)] Uploading to Storage Box..."
scp \
    -o StrictHostKeyChecking=no \
    "$BACKUP_DIR/lecoq_$DATE.dump" \
    "$STORAGE_BOX_USER@$STORAGE_BOX_HOST:/backup/lecoq/"

# Delete local backups older than retention
echo "[$(date -Iseconds)] Cleaning old local backups..."
find "$BACKUP_DIR" -name "lecoq_*.dump" -mtime +$RETENTION_DAYS -delete

# Delete remote backups older than retention (via SSH)
echo "[$(date -Iseconds)] Cleaning old remote backups..."
ssh "$STORAGE_BOX_USER@$STORAGE_BOX_HOST" \
    "find /backup/lecoq -name 'lecoq_*.dump' -mtime +$RETENTION_DAYS -delete"

echo "[$(date -Iseconds)] Backup completed: lecoq_$DATE.dump"
```

**Permissions :**
```bash
chmod 700 /home/yaya/scripts/backup-lecoq.sh
chown yaya:yaya /home/yaya/scripts/backup-lecoq.sh
```

**Cron :**
```bash
# Crontab (every day at 3am)
crontab -e

# Add:
0 3 * * * /home/yaya/scripts/backup-lecoq.sh >> /var/log/lecoq-backup.log 2>&1
```

**Test de restore (mensuel) :**
```bash
# Create test database
createdb lecoq_test

# Restore
pg_restore \
    -U lecoq \
    -d lecoq_test \
    /backup/lecoq/lecoq_2026-07-21_03-00-00.dump

# Verify
psql -U lecoq -d lecoq_test -c "SELECT COUNT(*) FROM entities;"

# Cleanup
dropdb lecoq_test
```

---

### Phase 2 (WAL Archiving + pgBackRest)

**Configuration PostgreSQL :**
```sql
-- postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'pgbackrest --stanza=lecoq archive-push %p'
max_wal_senders = 3
wal_compression = on
```

**pgBackRest config :**
```ini
# /etc/pgbackrest/pgbackrest.conf
[global]
repo1-path=/var/lib/pgbackrest
repo1-retention-full=2
repo1-retention-diff=7
repo1-cipher-type=aes-256-cbc
repo1-cipher-pass=<cipher-pass>

[lecoq]
db1-path=/var/lib/postgresql/16/main
db1-user=postgres
```

**Backup schedule :**
```bash
# Full backup weekly (Sunday 2am)
0 2 * * 0 pgbackrest --stanza=lecoq backup --type=full

# Differential backup daily (other days 2am)
0 2 * * 1-6 pgbackrest --stanza=lecoq backup --type=diff

# WAL archiving continuous (via archive_command)
```

**Restore :**
```bash
# Point-in-time recovery
pgbackrest --stanza=lecoq restore \
    --type=time \
    --target="2026-07-21 10:00:00"
```

---

## 📊 Monitoring

### Phase 1 (Logs + /health)

**Health endpoint (Rust) :**
```rust
// backend/src/routes/health.rs
use axum::{Json, http::StatusCode};
use serde_json::json;
use sqlx::PgPool;
use std::time::Instant;

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    timestamp: String,
    version: &'static str,
    database: DatabaseHealth,
    uptime_secs: u64,
}

#[derive(serde::Serialize)]
struct DatabaseHealth {
    status: &'static str,
    connections: u32,
    max_connections: u32,
    latency_ms: f64,
}

pub async fn health_check(
    db: PgPool,
    start_time: axum::extract::State<Instant>,
) -> Json<HealthResponse> {
    let start = Instant::now();
    
    // Check database connection
    let db_status = match sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&db)
        .await
    {
        Ok(_) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            DatabaseHealth {
                status: "ok",
                connections: db.size() as u32,
                max_connections: db.max_connections(),
                latency_ms: latency,
            }
        }
        Err(_) => DatabaseHealth {
            status: "error",
            connections: 0,
            max_connections: 0,
            latency_ms: 0.0,
        },
    };
    
    let uptime = start_time.elapsed().as_secs();
    
    let status = if db_status.status == "ok" {
        "ok"
    } else {
        "degraded"
    };
    
    Json(HealthResponse {
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION"),
        database: db_status,
        uptime_secs: uptime,
    })
}
```

**Logging (tracing + JSON) :**
```rust
// backend/src/main.rs
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn setup_logging() {
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .json();  // JSON format for log aggregation
    
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,lecoq=debug"));
    
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}
```

**Log rotation (logrotate) :**
```bash
# /etc/logrotate.d/lecoq
/var/log/lecoq/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 yaya yaya
    postrotate
        systemctl reload lecoq-backend
    endscript
}
```

**Dashboard simple (terminal) :**
```bash
#!/bin/bash
# /home/yaya/scripts/lecoq-status.sh

echo "=== Le Coq Status ==="
echo ""

# Backend process
echo "Backend:"
if pgrep -f "lecoq-backend" > /dev/null; then
    echo "  ✅ Running (PID: $(pgrep -f lecoq-backend))"
else
    echo "  ❌ Not running"
fi

# Database
echo ""
echo "Database:"
if pg_isready -h localhost -U lecoq > /dev/null 2>&1; then
    echo "  ✅ PostgreSQL ready"
    psql -h localhost -U lecoq -d lecoq -t -c \
        "SELECT '  Entities: ' || COUNT(*) FROM entities;" 2>/dev/null
    psql -h localhost -U lecoq -d lecoq -t -c \
        "SELECT '  Relationships: ' || COUNT(*) FROM relationships;" 2>/dev/null
else
    echo "  ❌ PostgreSQL not ready"
fi

# Disk usage
echo ""
echo "Disk:"
df -h /var/lib/postgresql | awk 'NR==2 {print "  PostgreSQL: " $3 " / " $2 " (" $5 " used)"}'

# Last backup
echo ""
echo "Last backup:"
ls -lht /backup/lecoq/*.dump 2>/dev/null | head -1 | awk '{print "  " $9 " (" $5 ")"}'

echo ""
```

---

### Phase 2 (Prometheus + Grafana)

**Prometheus metrics (Rust) :**
```rust
// backend/src/metrics.rs
use axum::{body::Body, http::Request};
use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use tower_http::trace::TraceLayer;
use std::time::Duration;

pub fn setup_metrics() {
    // Prometheus exporter
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9000))
        .install()
        .unwrap();
    
    // Track request latency
    let latency_buckets = vec![
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];
    
    histogram!("request_latency_seconds", latency_buckets.clone());
    counter!("requests_total");
    counter!("errors_total");
}

// Middleware to track metrics
pub async fn metrics_middleware(
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    let path = request.uri().path().to_string();
    let start = Instant::now();
    
    let response = next.run(request).await;
    
    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();
    
    counter!("requests_total", 1, "path" => path.clone());
    histogram!("request_latency_seconds", latency, "path" => path.clone());
    
    if status >= 500 {
        counter!("errors_total", 1, "path" => path, "status" => status.to_string());
    }
    
    response
}
```

**Prometheus config :**
```yaml
# /etc/prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'lecoq-backend'
    static_configs:
      - targets: ['localhost:9000']
    metrics_path: /metrics

  - job_name: 'postgres'
    static_configs:
      - targets: ['localhost:9187']  # postgres_exporter

  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']  # node_exporter
```

**Grafana dashboard (JSON import) :**
- Dashboard ID: 10001 (Rust application)
- Dashboard ID: 9628 (PostgreSQL overview)
- Dashboard ID: 1860 (Node exporter)

**Alerts (Alertmanager) :**
```yaml
# /etc/prometheus/alerts.yml
groups:
  - name: lecoq
    rules:
      - alert: BackendDown
        expr: up{job="lecoq-backend"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Le Coq backend is down"
          description: "Backend has been down for more than 1 minute"

      - alert: HighErrorRate
        expr: rate(errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors/sec"

      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(request_latency_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "95th percentile latency is {{ $value }}s"

      - alert: DatabaseDown
        expr: pg_up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "PostgreSQL is down"
          description: "Database has been down for more than 1 minute"
```

**Notifications (Slack) :**
```yaml
# /etc/alertmanager/alertmanager.yml
route:
  receiver: slack-notifications
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h

receivers:
  - name: slack-notifications
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/XXX/YYY/ZZZ'
        channel: '#lecoq-alerts'
        send_resolved: true
        title: '{{ .Status | toUpper }}: {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

---

### Phase 3 (ELK/Loki)

**Loki config :**
```yaml
# /etc/loki/local-config.yaml
auth_enabled: false

server:
  http_listen_port: 3100

common:
  path_prefix: /var/lib/loki
  storage:
    filesystem:
      chunks_directory: /var/lib/loki/chunks
      rules_directory: /var/lib/loki/rules

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h

limits_config:
  retention_period: 168h  # 7 days
```

**Promtail config :**
```yaml
# /etc/promtail/config.yml
server:
  http_listen_port: 9080

positions:
  filename: /var/lib/promtail/positions.yaml

clients:
  - url: http://localhost:3100/loki/api/v1/push

scrape_configs:
  - job_name: lecoq-backend
    static_configs:
      - targets:
          - localhost
        labels:
          job: lecoq-backend
          __path__: /var/log/lecoq/*.log
```

**Grafana Loki queries :**
```
# Errors in last hour
{job="lecoq-backend"} |= "ERROR" | __error__=""

# Latency > 1s
{job="lecoq-backend"} |= "latency" |~ "[1-9][0-9]{2,}ms"

# API key authentication failures
{job="lecoq-backend"} |= "UNAUTHORIZED" |= "X-API-Key"
```

---

## 🚀 CI/CD

### Phase 1 (GitHub Actions — tests + build)

**Workflow CI :**
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_VERSION: 1.75
  PYTHON_VERSION: 3.11

jobs:
  test-backend:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: lecoq
          POSTGRES_PASSWORD: test
          POSTGRES_DB: lecoq_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ env.RUST_VERSION }}
          override: true
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libssl-dev pkg-config
      
      - name: Build
        run: cargo build --release
        working-directory: backend
      
      - name: Run tests
        run: cargo test -- --test-threads=1
        working-directory: backend
        env:
          DATABASE_URL: postgresql://lecoq:test@localhost:5432/lecoq_test
      
      - name: Security Audit
        run: cargo audit
        working-directory: backend
      
      - name: Clippy (lint)
        run: cargo clippy -- -D warnings
        working-directory: backend
      
      - name: Format check
        run: cargo fmt -- --check
        working-directory: backend

  test-plugins:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: ${{ env.PYTHON_VERSION }}
      
      - name: Install dependencies
        run: |
          cd plugins
          pip install -r requirements.txt
          pip install pytest pytest-cov
      
      - name: Run tests
        run: |
          cd plugins
          pytest --cov=. tests/
      
      - name: Safety check
        run: |
          cd plugins
          pip install safety
          safety check -r requirements.txt

  build-docker:
    needs: [test-backend, test-plugins]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2
      
      - name: Login to Hetzner Registry
        uses: docker/login-action@v2
        with:
          registry: registry.hetzner.com
          username: ${{ secrets.HETZNER_REGISTRY_USER }}
          password: ${{ secrets.HETZNER_REGISTRY_PASSWORD }}
      
      - name: Build and push backend
        uses: docker/build-push-action@v4
        with:
          context: ./backend
          push: true
          tags: |
            registry.hetzner.com/lecoq/backend:latest
            registry.hetzner.com/lecoq/backend:${{ github.sha }}
          cache-from: type=registry,ref=registry.hetzner.com/lecoq/backend:buildcache
          cache-to: type=registry,ref=registry.hetzner.com/lecoq/backend:buildcache,mode=max
      
      - name: Build and push plugins
        uses: docker/build-push-action@v4
        with:
          context: ./plugins
          push: true
          tags: |
            registry.hetzner.com/lecoq/plugins:latest
            registry.hetzner.com/lecoq/plugins:${{ github.sha }}
```

---

### Phase 2 (Auto-deploy)

**Workflow Deploy :**
```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]
  workflow_dispatch:  # Manual trigger

env:
  HETZNER_HOST: ${{ secrets.HETZNER_HOST }}
  HETZNER_USER: ${{ secrets.HETZNER_USER }}

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Deploy to Hetzner
        uses: appleboy/ssh-action@master
        with:
          host: ${{ env.HETZNER_HOST }}
          username: ${{ env.HETZNER_USER }}
          key: ${{ secrets.HETZNER_SSH_KEY }}
          script: |
            set -e
            
            echo "=== Deploying Le Coq ==="
            
            cd /opt/lecoq
            
            # Pull latest images
            echo "Pulling latest images..."
            docker compose pull
            
            # Run database migrations
            echo "Running migrations..."
            docker compose run --rm backend ./run-migrations.sh
            
            # Restart services (rolling update)
            echo "Restarting services..."
            docker compose up -d --no-deps backend
            docker compose up -d --no-deps plugins
            
            # Health check
            echo "Waiting for backend to be healthy..."
            for i in {1..30}; do
              if curl -f http://localhost:8080/health > /dev/null 2>&1; then
                echo "✅ Backend is healthy"
                exit 0
              fi
              echo "Waiting... ($i/30)"
              sleep 2
            done
            
            echo "❌ Backend failed to start"
            exit 1
      
      - name: Notify success
        if: success()
        uses: 8398a7/action-slack@v3
        with:
          status: custom
          custom_payload: |
            {
              "text": "✅ Le Coq deployed successfully",
              "attachments": [{
                "color": "good",
                "fields": [{
                  "title": "Commit",
                  "short": true,
                  "value": "<${{ github.event.head_commit.url }}|${{ github.event.head_commit.id }}>"
                }, {
                  "title": "Author",
                  "short": true,
                  "value": "${{ github.actor }}"
                }]
              }]
            }
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
      
      - name: Notify failure
        if: failure()
        uses: 8398a7/action-slack@v3
        with:
          status: custom
          custom_payload: |
            {
              "text": "❌ Le Coq deployment failed",
              "attachments": [{
                "color": "danger",
                "fields": [{
                  "title": "Commit",
                  "short": true,
                  "value": "<${{ github.event.head_commit.url }}|${{ github.event.head_commit.id }}>"
                }, {
                  "title": "Author",
                  "short": true,
                  "value": "${{ github.actor }}"
                }]
              }]
            }
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
```

---

### Phase 3 (Rollback automatique)

**Rollback script :**
```bash
#!/bin/bash
# /home/yaya/scripts/rollback-lecoq.sh

set -euo pipefail

if [ -z "$1" ]; then
    echo "Usage: $0 <commit-sha>"
    exit 1
fi

COMMIT_SHA=$1
echo "Rolling back to $COMMIT_SHA..."

cd /opt/lecoq

# Pull specific version
docker pull registry.hetzner.com/lecoq/backend:$COMMIT_SHA
docker pull registry.hetzner.com/lecoq/plugins:$COMMIT_SHA

# Update docker-compose.yml
sed -i "s/image: registry.hetzner.com\/lecoq\/backend:latest/image: registry.hetzner.com\/lecoq\/backend:$COMMIT_SHA/" docker-compose.yml
sed -i "s/image: registry.hetzner.com\/lecoq\/plugins:latest/image: registry.hetzner.com\/lecoq\/plugins:$COMMIT_SHA/" docker-compose.yml

# Restart
docker compose up -d

# Health check
for i in {1..30}; do
    if curl -f http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ Rollback successful"
        exit 0
    fi
    echo "Waiting... ($i/30)"
    sleep 2
done

echo "❌ Rollback failed"
exit 1
```

---

## 📋 Checklist par Phase

### Phase 1 (MVP)

- [ ] **Backup**
  - [ ] Script pg_dump (/home/yaya/scripts/backup-lecoq.sh)
  - [ ] Cron quotidien (3am)
  - [ ] Test de restore mensuel
  - [ ] Hetzner Storage Box configuré

- [ ] **Monitoring**
  - [ ] Health endpoint (/health)
  - [ ] Logs JSON (tracing)
  - [ ] Log rotation (logrotate)
  - [ ] Script status (terminal dashboard)

- [ ] **CI/CD**
  - [ ] GitHub Actions (CI workflow)
  - [ ] Tests auto (backend + plugins)
  - [ ] Security audit (cargo audit, safety)
  - [ ] Build Docker auto
  - [ ] Deploy manuel (SSH)

---

### Phase 2 (SaaS)

- [ ] **Backup**
  - [ ] WAL archiving (pgBackRest)
  - [ ] PITR (point-in-time recovery)
  - [ ] Backup différentiel quotidien

- [ ] **Monitoring**
  - [ ] Prometheus (métriques custom)
  - [ ] Grafana (dashboards)
  - [ ] Alertmanager (alerts Slack)
  - [ ] Node exporter (système)
  - [ ] Postgres exporter (DB)

- [ ] **CI/CD**
  - [ ] Auto-deploy (GitHub Actions → Hetzner)
  - [ ] Health check post-deploy
  - [ ] Notifications Slack (succès/échec)
  - [ ] Environment protection (main branch)

---

### Phase 3 (Enterprise)

- [ ] **Backup**
  - [ ] Backup encryption (AES-256)
  - [ ] Backup géo-redondant (2 sites)
  - [ ] RTO/RPO testés trimestriellement

- [ ] **Monitoring**
  - [ ] Loki (logs centralisés)
  - [ ] ELK (recherche avancée)
  - [ ] Distributed tracing (Jaeger)
  - [ ] SLO/SLI tracking

- [ ] **CI/CD**
  - [ ] Rollback automatique (health check fail)
  - [ ] Canary deployment (5% → 100%)
  - [ ] Blue-green deployment
  - [ ] Performance tests auto

---

## 🔐 Secrets (GitHub Actions)

**À configurer dans GitHub Repo Settings → Secrets :**

```
HETZNER_HOST=api.lecoq.fr
HETZNER_USER=yaya
HETZNER_SSH_KEY=<clé privée SSH>
HETZNER_REGISTRY_USER=u123456
HETZNER_REGISTRY_PASSWORD=<mot de passe registry>
SLACK_WEBHOOK_URL=https://hooks.slack.com/...
```

---

## 📚 Références

- [PostgreSQL Backup](https://www.postgresql.org/docs/16/backup.html)
- [pgBackRest](https://pgbackrest.org/)
- [Prometheus](https://prometheus.io/)
- [Grafana](https://grafana.com/)
- [GitHub Actions](https://docs.github.com/en/actions)
- [Hetzner Storage Box](https://docs.hetzner.com/storage-boxes/)

---

*Fabriqué en France 🇫🇷*  
*Souverain. Transparent. Puissant.*  
*AGPL-3.0 — Stays open forever*
