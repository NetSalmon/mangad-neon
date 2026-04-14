#!/usr/bin/env bash
set -euo pipefail

# 检查 psql 命令是否存在
if command -v psql > /dev/null 2>&1; then
  echo "psql exist"
else
  echo "psql not exist"
  exit 1
fi

read -s -p "Enter Postgres password: " PG_PASSWORD
export PGPASSWORD="$PG_PASSWORD"
echo ""

PG_HOST="${PG_HOST:-localhost}"
PG_USER="${PG_USER:-postgres}"
PG_DB="${PG_DB:-manga_neon}"

if [[ ! -f "./migrate.sql" ]]; then
  echo "sql not exist"
  exit 1
fi

echo "Checking if database ${PG_DB} exists..."
DB_EXISTS=$(psql -h "$PG_HOST" -U "$PG_USER" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '${PG_DB}'")

if [ "$DB_EXISTS" = '1' ]; then
  echo "Database ${PG_DB} already exists, skip creating"
else
  echo "Creating database ${PG_DB}..."
  psql -h "$PG_HOST" -U "$PG_USER" -d postgres -c "CREATE DATABASE ${PG_DB};"
fi

echo "${PG_DB} migrating..."
psql -h "$PG_HOST" -U "$PG_USER" -d "$PG_DB" -f ./init/init.sql
echo "all done"