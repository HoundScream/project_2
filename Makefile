SHELL := /bin/bash

export CARGO_HOME ?= $(HOME)/.cargo
export PATH := $(CARGO_HOME)/bin:$(PATH)

.PHONY: rust-check rust-run rust-fmt rust-clippy rust-build-release db-up db-down db-logs db-psql docker-up docker-down docker-logs healthcheck browser-worker-setup sqlx-cli-install db-migrate-run db-migrate-add scenario-test scenario-run scenario-test-browser

rust-check:
	cargo check

rust-run:
	cargo run

rust-fmt:
	cargo fmt

rust-clippy:
	cargo clippy --all-targets --all-features -- -D warnings

rust-build-release:
	cargo build --release

db-up:
	docker compose up -d postgres

docker-up:
	docker compose up -d --build

db-down:
	docker compose down

docker-down:
	docker compose down

db-logs:
	docker compose logs -f postgres

docker-logs:
	docker compose logs -f app postgres

db-psql:
	docker compose exec postgres psql -U postgres -d geonetix

healthcheck:
	./scripts/healthcheck.sh

browser-worker-setup:
	npm ci
	npx playwright install --with-deps chromium
	npm run browser-worker:check

sqlx-cli-install:
	cargo install sqlx-cli --no-default-features --features rustls,postgres

db-migrate-run:
	sqlx migrate run --source migrations

db-migrate-add:
	sqlx migrate add --source migrations $(name)

# Требуют уже запущенного сервера (make rust-run в соседнем терминале).
# Формат сценариев и подробности: scenarios/README.md
scenario-test:
	cargo test --test run_scenarios -- --nocapture

scenario-run:
	cargo run --bin scenario-runner -- scenarios/*.json

scenario-test-browser:
	GEONETIX_BROWSER_NO_SANDBOX=1 node ui-tests/runner.mjs --screenshots scenarios/*.json
