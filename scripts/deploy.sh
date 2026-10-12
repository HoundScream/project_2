#!/usr/bin/env bash

set -euo pipefail

cd /opt/geonetix

docker compose pull redis caddy
docker compose up -d --build --remove-orphans --force-recreate app redis caddy
docker compose ps
docker image prune -f
