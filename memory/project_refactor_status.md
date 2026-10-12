---
name: project-refactor-status
description: Текущий статус работ по рефакторингу Geonetix — приоритеты и место остановки
metadata:
  type: project
---

Провели полный аудит архитектуры проекта. Составили приоритизированный план действий.

**Why:** main.rs на 10 396 строк — технический долг, который блокирует всё дальнейшее развитие.

**How to apply:** При возобновлении работы начинать с пункта 1 — разбивки main.rs.

## Место остановки (2026-07-01) — задача 6 завершена (код не проверен компилятором в этой среде)

**ЗАДАЧА 6 ЗАВЕРШЕНА** (с оговоркой). Redis подключён под кэш live-парсинга карточек (canonical_url → LiveMapCardData), TTL по умолчанию 900с (`GEONETIX_LIVE_CARD_CACHE_TTL_SECONDS`).

**ВАЖНО:** в этой рабочей среде нет доступа к `index.crates.io` (только к docs.rs напрямую), поэтому `cargo check`/`cargo build` с новой зависимостью `redis` здесь не проходят (зависают на резолвинге реестра). Сигнатуры API (`AsyncCommands::get`, `set_ex`, `Client::open`, `get_connection_manager`, `ConnectionManager: Clone`, `IntoConnectionInfo for String`) перепроверены вручную через docs.rs 0.27.0 — совпадают с использованными в коде, но **реальный `cargo build` нужно прогнать на машине/CI с доступом к crates.io перед мёржем**, `Cargo.lock` там же обновится автоматически (сейчас в нём ещё нет записей `redis`).

Что сделано:
- `Cargo.toml` — `redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }`.
- `.env.example` — `REDIS_URL`, `DOCKER_REDIS_URL`, `GEONETIX_LIVE_CARD_CACHE_TTL_SECONDS=900`.
- `docker-compose.yml` (локальный) — добавлен сервис `redis` (redis:7.4-alpine, healthcheck), `app` теперь зависит от его health и получает `REDIS_URL`.
- `deploy/docker-compose.prod.yml` — сервис `redis` уже был, добавлена явная `REDIS_URL: ${REDIS_URL:-redis://redis:6379}` для `app` (раньше контейнер был занесён в compose, но код к нему не обращался).
- `src/models.rs` — `AppState.redis: redis::aio::ConnectionManager`; `LiveMapCardData` получил `Serialize`/`Deserialize` (нужно для JSON-сериализации в кэш).
- `src/utils.rs` — `live_card_cache_ttl_seconds()`.
- `src/analysis.rs` — `collect_live_map_card_data` теперь: сначала `GET live_card:{platform}:{canonical_url}` из Redis, при попадании возвращает закэшированные данные (с пометкой в diagnostics "взято из кэша Redis"); при промахе выполняет обычный путь (browser-worker или HTML-фолбэк) и потом `SET EX` с TTL. Ошибки Redis не валят анализ (`if let Ok(...)`, кэш — это оптимизация, а не обязательное условие). `redis: &redis::aio::ConnectionManager` протянут через `run_analysis_engine[_inner]` и `run_module_eight_audit[_inner]` теми же by-value/by-ref парами, что и `pool: PgPool`/`&PgPool` — для консистентности со сложившимся стилем.
- `src/handlers.rs` — все 3 места `tokio::spawn(run_analysis_engine/run_module_eight_audit(...))` передают `state.redis.clone()`.
- `src/main.rs` — подключение к Redis (`redis::Client::open(REDIS_URL)?.get_connection_manager().await?`) перед стартом сервера, `AppState { pool, redis, report_storage_dir }`.

Следующий шаг при возобновлении: прогнать `cargo build`/`cargo check` там, где есть сеть до crates.io, поднять `docker compose up -d --build` локально и вручную проверить, что повторный запуск анализа той же карточки отдаёт "взято из кэша Redis" в диагностике.

## Архивная запись (2026-07-01) — задача 4 завершена

**ЗАДАЧА 4 ЗАВЕРШЕНА.** Нормализация billing-полей.

Решение по неймингу (согласовано с пользователем): `free_analysis_limit`/`free_analysis_used` → `analysis_quota_limit`/`analysis_quota_used`. Причина переименования: поле уже переиспользуется и для free, и для pro-тарифов (при `activate_paid_subscription` limit перезаписывается, used сбрасывается в 0) — старое имя с "free" вводило в заблуждение.

Что сделано:
- `migrations/0002_rename_billing_quota_fields.sql` — новая миграция (0001 не трогали, т.к. уже применена и её чек-сумма зафиксирована): `rename column` для обоих полей, `rename constraint` для обоих check-констрейнтов, плюс добавлен nullable `analysis_quota_resets_at timestamptz` — placeholder на будущее (когда придёт платёжный шлюз, туда можно будет писать дату сброса квоты за период). Столбец пока нигде не читается/не пишется в Rust-коде — сознательно, чтобы не тащить недоделанную логику вперёд задачи 5.
- `src/models.rs` — поля `RegisteredUser` переименованы.
- `src/db.rs` — `fetch_session_user`, `fetch_registered_user`, `activate_paid_subscription` используют новые имена колонок.
- `src/handlers.rs` — insert при регистрации, select в `register_success`, увеличение счётчика в `create_analysis_request` (`analysis_quota_used = analysis_quota_used + 1`).
- `src/utils.rs` — `validate_subscription_access` сравнивает `analysis_quota_used >= analysis_quota_limit`.
- `src/pages.rs` — все шаблонные плейсхолдеры и `format!`-байндинги в `render_cabinet_page`/`render_subscription_page` переименованы; видимый пользователю текст не менялся (уже был нейтральным на русском — "лимит запусков").
- Проверено вживую на локальном Postgres: после `cargo run` `_sqlx_migrations` показывает 2 записи (`init`, `rename billing quota fields`), `\d app_users` подтверждает новые имена колонок и констрейнтов, старых `free_analysis_*` нигде не осталось (`grep` по `src/` — пусто).
- `cargo check` чист (только старые dead-code warnings).

## Архивная запись (2026-07-01) — задача 3 завершена

**ЗАДАЧА 3 ЗАВЕРШЕНА.** sqlx migrations вместо `ensure_analysis_schema`.

Что сделано:
- `Cargo.toml` — добавлена фича `migrate` в sqlx.
- `migrations/0001_init.sql` — единая консолидированная миграция: все базовые таблицы (`app_healthcheck`, `geo_objects`, `app_users`, `app_sessions`, `business_profiles`, `analysis_requests`, `analysis_reports`, `pro_monitoring_scenarios`, `pro_monitoring_events`, `module_eight_audits`, `module_nine_reports`) + все накопленные `alter table add column if not exists` + условный constraint (`do $$ ... $$`) + one-time backfill `update app_users`. Написана в идемпотентном стиле (`if not exists` везде), поэтому безопасна и для чистой БД, и для уже существующей прод-БД со старой схемой.
- `database/init/001_init.sql` — урезан до одной строки `create extension if not exists "uuid-ossp"`; схема теперь полностью живёт в `migrations/`.
- `src/main.rs` — `db::ensure_analysis_schema(&pool)` заменён на `sqlx::migrate!("./migrations").run(&pool).await?`, вызов перенесён ДО healthcheck-запроса (раньше healthcheck читал `app_healthcheck` ещё до создания схемы — что работало только потому что таблицы уже существовали от старого бутстрапа).
- `src/db.rs` — функция `ensure_analysis_schema` (219 строк) удалена полностью.
- `Makefile` — новые таргеты `sqlx-cli-install`, `db-migrate-run`, `db-migrate-add`.
- Проверено вживую: локальный Postgres (localhost:5432/geonetix) уже содержал таблицы от старого `ensure_analysis_schema`. `cargo run` успешно применил `migrations/0001_init.sql` к этой существующей БД без ошибок (idempotent create/alter сработал как no-op), создал таблицу `_sqlx_migrations` (1 запись, version=1, success=true), healthcheck вернул 3 строки. Повторный запуск не создал дублей (`app_healthcheck` всё ещё 3 строки, `_sqlx_migrations` всё ещё 1 запись) — миграция не переигрывается.
- `cargo check` чист (только старые dead-code warnings).

Дальше для новых изменений схемы: `make db-migrate-add name=<name>` создаёт новый файл в `migrations/`, миграции применяются автоматически при следующем старте приложения (`sqlx::migrate!` встроен в бинарь на этапе компиляции, `DATABASE_URL`/сеть на этапе сборки не нужны, т.к. в проекте нет compile-time `sqlx::query!`/`query_as!`).

## Архивная запись (2026-07-01) — задача 2 завершена

**ЗАДАЧА 2 ЗАВЕРШЕНА.** Подключение browser-worker.

Что сделано:
- `noSandboxEnabled()`/`defaultLaunchArgs()` добавлены в `workers/yandex_browser_worker.mjs` — при `GEONETIX_BROWSER_NO_SANDBOX=1` передаются `--no-sandbox --disable-setuid-sandbox` в `chromium.launch()`.
- `Dockerfile` обновлён: в финальном `debian:bookworm-slim` стейдже ставится Node.js 20 (через NodeSource), копируются `package.json`/`package-lock.json`/`workers/`, выполняется `npm ci --omit=dev` и `npx playwright install --with-deps chromium`. Добавлен `ENV GEONETIX_BROWSER_NO_SANDBOX=1` (нужен в контейнере, где root без sandbox не работает).
- `.env.example` — добавлена строка `GEONETIX_BROWSER_NO_SANDBOX=0` (локально sandbox не мешает).
- `Makefile` — новый таргет `browser-worker-setup` (npm ci + playwright install + browser-worker:check).
- `node --check` прошёл; тестовый прогон скрипта (`GEONETIX_BROWSER_NO_SANDBOX=1` и без флага) отработал end-to-end и вернул валидный JSON (ожидаемая ошибка "card state not found" — тестовый URL не настоящая карточка, но браузер запустился и не упал).
- `cargo check` проходит без ошибок (только старые dead-code warnings, не связанные с этой задачей).

Не проверено: полная сборка Docker-образа (`docker` недоступен в этой среде — нет бинарника). Если будет доступ к Docker, стоит прогнать `docker build .` перед деплоем.

**ЗАДАЧА 1 ЗАВЕРШЕНА.** `cargo build` проходит без ошибок.

Итог разбивки:
- `src/models.rs` — все типы/структуры (~456 строк)
- `src/utils.rs` — утилиты, label-функции, validation (~870 строк)
- `src/db.rs` — все запросы к БД, PDF-генерация (~1500 строк)
- `src/pages.rs` — все render_* функции (~3860 строк)
- `src/analysis.rs` — движки анализа module 8 и основного (~1380 строк)
- `src/handlers.rs` — HTTP-хендлеры, роутинг (~1720 строк)
- `src/main.rs` — только main() и роутер (149 строк)

Функции `health`, `search` и `render_analysis_status_page` были потеряны при сплите и восстановлены из контекста.

## Приоритизированный план (с чего начинать)

1. **Разбить `src/main.rs` на модули** ✅ ЗАВЕРШЕНО
   - Целевая структура:
     - `src/main.rs` — только main() и роутер
     - `src/state.rs` — AppState, константы
     - `src/models.rs` — все structs/enums
     - `src/db/` — users, analysis, monitoring, module_eight, module_nine
     - `src/handlers/` — public, auth, cabinet, pro
     - `src/pages/` — все render_* функции
     - `src/analysis/` — engine, module_eight движок
     - `src/utils/` — validate, normalize, hash, cookies, parse, fetch
   - После: `cargo check` должен проходить

2. **Подключить реальный browser-worker** (workers/yandex_browser_worker.mjs уже написан)
   - Проверить путь `collect_live_map_card_data_with_browser_worker` в main.rs
   - Протестировать с `GEONETIX_BROWSER_WORKER_ENABLED=1`

3. **Ввести sqlx migrations** вместо `ensure_analysis_schema` ✅ ЗАВЕРШЕНО

4. **Нормализовать billing-поля** ✅ ЗАВЕРШЕНО (`analysis_quota_limit`/`analysis_quota_used`)

5. **Подключить реальный платёжный шлюз** (ЮKassa / Робокасса)
   - Модуль 13 сейчас симулирует оплату

6. **Прояснить роль Redis** ✅ ЗАВЕРШЕНО (кэш live-парсинга карточек; требует `cargo build` с доступом к crates.io перед мёржем — см. запись выше)
