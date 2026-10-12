#!/usr/bin/env bash
set -euo pipefail

if ! systemctl is-active --quiet postgresql; then
  echo "postgresql service is not active"
  exit 1
fi

if ! systemctl is-active --quiet geonetix; then
  echo "geonetix service is not active"
  exit 1
fi

database_url="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/geonetix}"

if ! PGPASSWORD="${POSTGRES_PASSWORD:-postgres}" psql "$database_url" -tAc "select 1" >/dev/null; then
  echo "postgresql query failed"
  exit 1
fi

if ! curl --fail --silent http://127.0.0.1/health >/dev/null; then
  echo "application health endpoint failed"
  exit 1
fi

echo "healthcheck ok"
