# Geonetix Rust + PostgreSQL

Минимальный HTTP-сервис на Rust с PostgreSQL, готовый для локального запуска и production-деплоя через Docker Compose.

## Что уже подготовлено

- `rustup`, `cargo`, `rustc` устанавливаются в домашнюю директорию пользователя
- проект на Cargo
- HTTP-приложение на `axum`
- `tokio`, `sqlx`, `dotenvy`, `anyhow`
- локальный `docker-compose.yml` для `app + postgres`
- `database/init/001_init.sql` для инициализации БД
- `Makefile` с основными командами

## Быстрый старт

```bash
. "$HOME/.cargo/env"
cp .env.example .env
cargo check
```

### Browser worker для Яндекс Карт

Опциональный worker на `Playwright` рендерит карточку в Chromium, открывает секцию отзывов, прокручивает контейнер для lazy-load и возвращает данные в Rust-пайплайн через JSON stdout. Он не маскирует автоматизацию под действия человека и не имитирует курсор.

Подготовка:

```bash
npm install
npx playwright install chromium
```

Чтобы включить worker, выставьте в `.env`:

```env
GEONETIX_BROWSER_WORKER_ENABLED=1
GEONETIX_BROWSER_WORKER_TIMEOUT_MS=45000
```

Если worker выключен или Playwright недоступен, приложение откатится на текущий HTML/state parser.

### Локальный запуск на хосте

```bash
docker compose up -d postgres
cargo run
```

После запуска:

- приложение отвечает на `http://127.0.0.1:8080/`
- healthcheck доступен на `http://127.0.0.1:8080/health`

### Полный локальный запуск в Docker

```bash
cp .env.example .env
docker compose up -d --build
curl http://127.0.0.1:8080/health
```

В этом режиме сервис `app` внутри Compose использует `DOCKER_DATABASE_URL`, который по умолчанию указывает на контейнер `postgres`.

## Полезные команды

```bash
make rust-check
make rust-run
make rust-build-release
make rust-fmt
make rust-clippy
make db-up
make db-down
make docker-up
make docker-down
make docker-logs
make healthcheck
```

## Автотесты пользовательских сценариев

Библиотека для воспроизведения пользовательских сценариев (регистрация, вход, запуск анализа, управление подпиской и т.д.) в целях автотестирования UI. Один и тот же сценарий, описанный в `scenarios/*.json`, можно прогнать двумя способами:

- **Rust / HTTP-уровень** (`src/scenario_lib`) — быстрый, без браузера, реальные HTTP-запросы к серверу с общим cookie-jar и проверкой отрендеренного HTML. Основной слой, подходит для CI.
- **Node / Playwright** (`ui-tests/`) — тяжёлый слой с настоящим headless-Chromium: реально кликает по формам и умеет делать скриншоты каждого шага в `artifacts/site-screenshots/`. Требует больше ресурсов машины.

Формат сценариев и полный список шагов/проверок — в [`scenarios/README.md`](scenarios/README.md).

Перед запуском нужен работающий сервер (`make rust-run` в соседнем терминале):

```bash
make scenario-test           # cargo test, все сценарии из scenarios/
make scenario-run            # то же самое через CLI-бинарник scenario-runner
make scenario-test-browser   # Playwright-слой со скриншотами
```

Прямое использование библиотеки из Rust-кода (не через JSON) — пример в `examples/direct_api_usage.rs` (`cargo run --example direct_api_usage`).

## Переменные окружения

- `DATABASE_URL` используется при запуске приложения на хосте
- `DOCKER_DATABASE_URL` используется локальным контейнером `app` и по умолчанию смотрит на сервис `postgres`
- `SERVER_NAME` и `APP_UPSTREAM` нужны production-стеку с Caddy

Пример:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/geonetix
DOCKER_DATABASE_URL=postgres://postgres:postgres@postgres:5432/geonetix
SERVER_NAME=geonetix.io
APP_UPSTREAM=app:8080
```

## Healthcheck

- `GET /` возвращает простой ответ `Geonetix is running`
- `GET /health` проверяет доступность PostgreSQL и возвращает `200 OK`
- production-container `app` и локальный Docker Compose используют `/health` как встроенную проверку живости

## Автозапуск через systemd

В репозитории есть шаблон сервиса:

- `deploy/geonetix.service`

Этот шаблон рассчитан на production-стек в Docker Compose. После установки сервиса и размещения файлов в `/opt/geonetix` стек будет подниматься после перезагрузки сервера, если доступны `docker`, `docker compose` и файл `/opt/geonetix/.env`.

## Автодеплой через GitHub Actions

Добавлены production-файлы:

- `.github/workflows/deploy.yml`
- `deploy/docker-compose.prod.yml`
- `deploy/Caddyfile`
- `deploy/deploy.sh`
- `Dockerfile`

Логика деплоя после каждого `push` в `main`:

1. GitHub Actions архивирует репозиторий.
2. Копирует архив на сервер по SSH.
3. Распаковывает в `/opt/geonetix`.
4. Очищает старые файлы релиза, сохраняя `/opt/geonetix/.env`.
5. Запускает `docker compose` для `app`, `caddy`, `redis`.
6. PostgreSQL используется хостовый (вне Docker) через `host.docker.internal:5432`.

### GitHub Secrets (Repository -> Settings -> Secrets and variables -> Actions)

- `DEPLOY_HOST` (пример: `159.194.205.155`)
- `DEPLOY_PORT` (обычно `22`)
- `DEPLOY_USER` (пример: `root`)
- `DEPLOY_SSH_KEY` (приватный ключ в формате OpenSSH)

### Что должно быть на сервере заранее

1. Установлены `docker` и `docker compose`.
2. Папка `/opt/geonetix`.
3. Файл `/opt/geonetix/.env` с `DATABASE_URL` к хостовому PostgreSQL, например:

```env
DATABASE_URL=postgres://postgres:YOUR_PASSWORD@host.docker.internal:5432/geonetix
SERVER_NAME=geonetix.io
APP_UPSTREAM=app:8080
```

### Что проверять после деплоя

```bash
docker compose -f /opt/geonetix/deploy/docker-compose.prod.yml --env-file /opt/geonetix/.env ps
curl -I https://geonetix.io
curl https://geonetix.io/health
```

### Быстрая проверка перед деплоем

Смотри отдельный список:

- `deploy/pre-deploy-checklist.md`
