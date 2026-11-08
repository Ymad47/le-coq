#!/bin/bash
# Le Coq - Daily Backup Script
# Cron example: 0 2 * * * /path/to/le-coq/scripts/backup.sh
# Defaults match docker/docker-compose.yml; override with env vars.
# Password: use ~/.pgpass or PGPASSWORD.

set -euo pipefail

# Configuration
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKUP_DIR="${BACKUP_DIR:-$ROOT/backups}"
PG_USER="${PG_USER:-postgres}"
PG_DB="${PG_DB:-le_coq}"
PG_HOST="${PG_HOST:-127.0.0.1}"
PG_PORT="${PG_PORT:-5432}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/lecoq_backup_$TIMESTAMP.sql.gz"

# Perform backup
echo "[$(date)] Starting backup..."
pg_dump -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" | gzip > "$BACKUP_FILE"

# Verify backup
if [ -f "$BACKUP_FILE" ] && [ -s "$BACKUP_FILE" ]; then
    echo "[$(date)] Backup successful: $BACKUP_FILE"
    echo "Size: $(du -h "$BACKUP_FILE" | cut -f1)"
else
    echo "[$(date)] Backup failed!"
    exit 1
fi

# Clean old backups
echo "[$(date)] Cleaning backups older than $RETENTION_DAYS days..."
find "$BACKUP_DIR" -name "lecoq_backup_*.sql.gz" -mtime +$RETENTION_DAYS -delete

echo "[$(date)] Backup complete."
