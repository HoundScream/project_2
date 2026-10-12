# Geonetix Pre-Deploy Checklist

Last updated: 2026-05-20

## Git and Repository

- `git status` is clean or only contains intentional changes
- current branch is `main`
- `origin` remote is configured correctly
- required files are present: `Dockerfile`, `deploy/docker-compose.prod.yml`, `deploy/deploy.sh`, `.github/workflows/deploy.yml`

## Local Validation

- `cargo check`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- if Docker is available locally: `docker build -t geonetix-app:test .`
- if Docker is available locally: `docker compose -f deploy/docker-compose.prod.yml config`

## GitHub Actions

- repository secrets are configured:
- `DEPLOY_HOST`
- `DEPLOY_PORT`
- `DEPLOY_USER`
- `DEPLOY_SSH_KEY`

## Server Readiness

- server path `/opt/geonetix` exists
- server file `/opt/geonetix/.env` exists
- `.env` contains valid `DATABASE_URL`, `SERVER_NAME`, and `APP_UPSTREAM`
- `docker` and `docker compose` are installed on the server
- DNS for `SERVER_NAME` points to the server
- ports `80` and `443` are reachable from the internet
- host PostgreSQL accepts connections from Docker via `host.docker.internal`

## Rollback Readiness

- previous known-good commit is identified
- database backup or restore plan exists before schema-changing deploys
- you know how to inspect logs:
- `docker compose -f /opt/geonetix/deploy/docker-compose.prod.yml logs -f app`
- `docker compose -f /opt/geonetix/deploy/docker-compose.prod.yml ps`
