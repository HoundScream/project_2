#!/usr/bin/env bash

set -euo pipefail

APP_DIR="${1:-/opt/geonetix}"

mkdir -p "${APP_DIR}/scripts"
chmod 755 "${APP_DIR}" "${APP_DIR}/scripts"

if command -v docker >/dev/null 2>&1; then
    docker --version
else
    echo "Docker is not installed on the server." >&2
    exit 1
fi

if docker compose version >/dev/null 2>&1; then
    docker compose version
else
    echo "Docker Compose plugin is not installed on the server." >&2
    exit 1
fi
