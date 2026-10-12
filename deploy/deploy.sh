#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/opt/geonetix"
COMPOSE_FILE="${APP_DIR}/deploy/docker-compose.prod.yml"
ENV_FILE="${APP_DIR}/.env"
ARCHIVE_PATH="${1:-}"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}. Create it on server before deploy."
  exit 1
fi

cd "${APP_DIR}"

echo "Starting deploy at $(date -u +'%Y-%m-%dT%H:%M:%SZ')"

if [[ -n "${ARCHIVE_PATH}" ]]; then
  if [[ ! -f "${ARCHIVE_PATH}" ]]; then
    echo "Archive ${ARCHIVE_PATH} not found."
    exit 1
  fi

  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "${TMP_DIR}"' EXIT

  tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"

  # Replace tracked app files while keeping server-only state such as .env.
  find "${APP_DIR}" -mindepth 1 -maxdepth 1 ! -name '.env' -exec rm -rf {} +
  cp -a "${TMP_DIR}"/. "${APP_DIR}"/
fi

docker compose -f "${COMPOSE_FILE}" --env-file "${ENV_FILE}" up -d --build --pull always --remove-orphans app redis caddy

docker image prune -f >/dev/null 2>&1 || true

docker compose -f "${COMPOSE_FILE}" ps
