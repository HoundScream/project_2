# Geonetix Server Notes

Last updated: 2026-05-20

## Access

- Hostname: `geonetix.io`
- Server IP: `159.194.205.155`
- SSH user currently used: `root`
- SSH port: `22`
- SSH key auth: working
- Local private key path used from this workstation: `~/.ssh/id_ed25519`
- Local public key path used from this workstation: `~/.ssh/id_ed25519.pub`

Public key installed on server:

```text
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPGddB7+mre5frttl8qqhvZl+K2FBeRL8DaLt6JSpjNC codex@geonetix
```

Notes:

- Several password-based login attempts were tested during setup.
- Passwords are intentionally not stored in this repository.
- SSH login by key was verified successfully for `root@159.194.205.155`.

## Server Summary

- Hostname: `oitbhegwzx`
- OS: `Ubuntu 24.04.4 LTS`
- Kernel: `Linux 6.8.0-110-generic`
- Virtualization: `kvm`
- Uptime at inspection: about `33 days`

## Resources

- RAM: `961 MiB`
- Memory in use during inspection: about `547 MiB`
- Available memory during inspection: about `413 MiB`
- Swap: not configured
- Root disk: `8.7G`
- Root disk used during inspection: `6.3G` (`73%`)

## Running Services

System services observed as running:

- `docker`
- `containerd`
- `postgresql@16-main`
- `ssh`
- `fail2ban`
- `NetworkManager`
- `unattended-upgrades`

Application stack observed:

- `geonetix-caddy-1`
- `geonetix-app-1`
- `geonetix-redis-1`

## Network and Ports

Listening ports observed during inspection:

- `22/tcp` -> `sshd`
- `80/tcp` -> Docker proxy / Caddy
- `443/tcp` -> Docker proxy / Caddy
- `5432/tcp` -> PostgreSQL on host

Docker-published services:

- `0.0.0.0:80 -> caddy`
- `0.0.0.0:443 -> caddy`

## Application Deployment

Compose file found at:

- `/opt/geonetix/docker-compose.yml`

Observed service layout:

- `app`: `geonetix-app:latest`
- `redis`: `redis:7.4-alpine`
- `caddy`: `caddy:2.8-alpine`

Application database connection observed from `/opt/geonetix/.env`:

- App connects to PostgreSQL via `host.docker.internal:5432`
- Database name: `geonetix`
- This indicates the app uses PostgreSQL running on the host, not inside Compose

Sensitive values from `.env` are intentionally not copied into this repository.

## SSH Configuration

Observed SSH-related settings:

- `/etc/ssh/sshd_config`: `PermitRootLogin yes`
- `/etc/ssh/sshd_config.d/50-cloud-init.conf`: `PasswordAuthentication yes`
- `/etc/ssh/sshd_config.d/60-cloudimg-settings.conf`: `PasswordAuthentication no`
- `/etc/ssh/sshd_config.d/99-disable-password-auth.conf`: `PasswordAuthentication no`
- `/etc/ssh/sshd_config.d/99-disable-password-auth.conf`: `PubkeyAuthentication yes`
- `/etc/ssh/sshd_config.d/99-disable-password-auth.conf`: `PermitRootLogin prohibit-password`

Effective outcome during testing:

- SSH access for `root` by key works
- Password-based SSH login for `root` was rejected

## PostgreSQL Notes

Observed PostgreSQL config:

- Config file: `/etc/postgresql/16/main/postgresql.conf`
- `listen_addresses = '*'`
- `pg_hba.conf` allows:
  - local peer auth
  - localhost SCRAM auth
  - Docker bridge network `172.17.0.0/16` via `scram-sha-256`

Validation performed:

- Local SQL check succeeded with `select 1;`

## Firewall Changes Applied

Goal:

- Close external access to PostgreSQL while preserving app access from Docker

Rules applied:

- Allow `172.17.0.0/16 -> tcp/5432`
- Drop other inbound traffic to `tcp/5432`

Validation:

- External check to `159.194.205.155:5432` became closed
- App container remained healthy
- PostgreSQL continued responding locally

Persistence:

- Installed `iptables-persistent`
- Installed `netfilter-persistent`
- Saved current iptables rules
- Enabled `netfilter-persistent.service`

Relevant persisted file:

- `/etc/iptables/rules.v4`

## Operational Notes

- `ufw` was not active at the time of inspection
- `fail2ban` was active
- `netfilter-persistent` is active and enabled

## GitHub Auto-Deploy Setup

Repository now contains:

- `.github/workflows/deploy.yml`
- `deploy/docker-compose.prod.yml`
- `deploy/Caddyfile`
- `deploy/deploy.sh`
- `Dockerfile`

Expected server path:

- `/opt/geonetix`

Required file on server before first deploy:

- `/opt/geonetix/.env`

Required GitHub Actions secrets (private repository):

- `DEPLOY_HOST` -> server IP or DNS
- `DEPLOY_PORT` -> SSH port (default `22`)
- `DEPLOY_USER` -> SSH user (currently `root`)
- `DEPLOY_SSH_KEY` -> private SSH key content

Deployment behavior on each push to `main`:

1. Workflow sends archive to `/tmp/geonetix.tar.gz` on server.
2. Archive is extracted into `/opt/geonetix`.
3. `deploy/deploy.sh` runs:
   - `docker compose up -d --build --pull always --remove-orphans app redis caddy`
4. PostgreSQL stays on host and is used through `host.docker.internal`.

## Recommended Follow-ups

- Consider adding swap for better stability during memory pressure
- Consider creating a separate admin user with `sudo` instead of routine `root` usage
- Consider documenting backup and restore procedure for PostgreSQL
- Keep secrets out of the repository and store them in a secure secret manager or password vault

## Product Follow-ups

- Подключить реальный платёжный шлюз для модуля 13 и общего PRO-потока, чтобы продление и смена тарифа происходили по подтверждённым платёжным событиям, а не через симуляцию обновления пользователя.
- Нормализовать модель лимитов: поля `free_analysis_limit` и `free_analysis_used` уже используются как общий лимит пакета, поэтому позже их лучше переименовать в нейтральные billing-поля или вынести в отдельную сущность биллинга/подписки.

---

## Module Development Workflow (Chat-by-Chat)

Purpose:

- Develop each module/block from the scheme in a separate chat.
- After each finished module, store a dedicated `.md` stage note.
- Keep a short summary of previous stages in every new stage note.

Storage location:

- `deploy/stages/`

File naming:

- `YYYY-MM-DD-stage-XX-module-name.md`
- Example: `2026-05-21-stage-01-auth-registration.md`

Required structure for each stage file:

1. Stage header:
   - stage number, module name, date, chat reference
2. Goal:
   - what exactly is implemented in this stage
3. Input context:
   - links/references to the scheme block and related files
4. Implementation summary:
   - key decisions
   - what was changed (files/components/endpoints)
5. Validation:
   - tests/checks that were run
   - result and known limitations
6. Outcome:
   - done/not done, what is ready for next stage
7. Short history (from previous stages):
   - 3-7 bullets with concise context from earlier completed stages
8. Next step:
   - exact next module or task in one sentence

Template:

```md
# Stage XX - <Module Name>

Date: YYYY-MM-DD
Chat: <short chat/topic reference>

## Goal
- <what we implement in this stage>

## Input Context
- Scheme block: <module/block name>
- Related files: <paths>

## Implementation Summary
- Decisions:
  - <decision 1>
  - <decision 2>
- Changes:
  - <file/path>: <what changed>

## Validation
- <command/check>: <result>
- <command/check>: <result>
- Limitations:
  - <known gap/risk>

## Outcome
- Status: <Done | Partial | Blocked>
- Ready for next stage: <yes/no + short note>

## Short History
- Stage 01: <1 line summary>
- Stage 02: <1 line summary>
- Stage 03: <1 line summary>

## Next Step
- <exact next module/task>
```
