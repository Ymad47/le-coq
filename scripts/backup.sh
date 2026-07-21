#!/bin/bash
# Le Coq - Daily Backup Script
# Runs via cron: 0 2 * * * /home/yaya/le-coq/scripts/backup.sh

set -e

# Configuration
BACKUP_DIR="/home/yaya/le-coq/backups"
PG_USER="lecoq"
PG_DB="lecoq"
PG_HOST="localhost"
PG_PORT="5432"
RETENTION_DAYS=7

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
