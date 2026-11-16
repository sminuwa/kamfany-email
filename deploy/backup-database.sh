#!/bin/sh
# Dump the desk database and keep 14 days of files.
# Offsite copy is a separate step. This script does not upload anywhere.
set -eu

if [ -z "${DATABASE_URL:-}" ]; then
  echo "DATABASE_URL is required" >&2
  exit 1
fi

dir="${BACKUP_DIR:-/var/backups/kamfany-mail}"
mkdir -p "$dir"
stamp=$(date -u +%Y%m%dT%H%M%SZ)
pg_dump --dbname="$DATABASE_URL" --format=custom --file="$dir/kamfany-mail-$stamp.dump"
find "$dir" -name 'kamfany-mail-*.dump' -type f -mtime +14 -delete
echo "wrote $dir/kamfany-mail-$stamp.dump"
