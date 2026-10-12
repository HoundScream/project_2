# Базовый функционал: shared Claude-ключ, поиск организаций — патч для Geonetix

## Важно: куда это накладывать

Этот пакет собран и проверен (`cargo check` проходит) против публичного
`HoundScream/project_2` на GitHub. Твоя рабочая копия в WSL (`/home/studytool/dev/Geonetix`)
по статусу от 5 октября уже ушла вперёд: там есть `src/pro/`, `src/analysis/` как
каталог, и миграции `0003_analytics_events.sql`, `0004_test_accounts.sql`,
`0005_free_quota_three.sql`, которых нет в GitHub-версии.

Поэтому:
1. **Не копируй файлы вслепую** — сначала проверь, что в твоём локальном
   репозитории нет таких же модулей (`src/admin.rs`, `src/ai_assist.rs`,
   `src/claude_client.rs`, `src/organizations.rs`, `src/settings.rs`).
2. **Миграцию переименуй.** Я назвал её `0006_app_settings_and_ai.sql`,
   предполагая, что у тебя уже заняты 0003–0005. Проверь `ls migrations`
   у себя и подставь следующий свободный номер.
3. Если выяснится, что что-то из уже сделанного (`account_role`,
   `AccountRole::bypasses_limits()`) конфликтует с логикой ниже — чужого
   не трогал специально, но тебе нужно свести руками.

## Что здесь сделано (и почему именно так — решения из нашего диалога)

Ты подтвердил: это для Geonetix, на Rust/axum (уже так и есть), ключ —
**общий, не BYOK** (платишь ты, у пользователей лимиты), поиск организаций —
**и внутри кабинета, и во внешнем справочнике**.

### 1. Общий Claude-ключ, а не ключ на пользователя

- `src/settings.rs` — таблица `app_settings` (key/value) + аудит изменений
  (`app_settings_audit`). Ключ и модель можно задать через `.env`
  (`CLAUDE_API_KEY`, `CLAUDE_MODEL`) или через `/admin/settings` — значение
  из базы имеет приоритет, так что можно сменить ключ без передеплоя.
- `src/claude_client.rs` — минимальный клиент Messages API. Я **намеренно
  не захардкодил имя модели** (твои данные по рынку устаревают быстрее,
  чем кажется) — его нужно явно задать, актуальный id бери на
  docs.claude.com/en/docs/about-claude/models.
- `src/admin.rs` — `GET/POST /admin/settings`, защищено Bearer-токеном
  (`ADMIN_TOKEN` в `.env`). Это заглушка, а не полноценная роль «админ» —
  в `app_users` такой роли пока нет (в статусе от 5 октября `account_role`
  только разрабатывается). Когда её сделаешь — замени проверку токена на
  проверку роли.
- `src/ai_assist.rs` — `POST /cabinet/{user_id}/ai/ask`: демонстрационный
  endpoint, который реально дергает Claude и тратит **ту же квоту**
  (`analysis_quota_used`), что и обычный аудит — отдельного лимита для ИИ
  я не заводил, чтобы не плодить вторую систему учёта.

### 2. Поиск организаций — обе части

- `src/organizations.rs::search_own_organizations` —
  `GET /cabinet/{user_id}/organizations/search?q=...`, поиск по уже
  добавленным `business_profiles` пользователя (по названию, адресу,
  ссылке). Требует валидной сессии именно этого `user_id` — чужие карточки
  не видны.
- `src/organizations.rs::lookup_external_organizations` —
  `GET /organizations/lookup?q=...`, поиск реальной компании по
  названию/ИНН через DaData (`suggest/party`). Без `DADATA_API_KEY`
  возвращает `501` с понятным текстом, а не падает.

## Что нужно решить тебе (не решал за тебя)

- Реальная роль «админ» вместо bearer-токена — когда будет `account_role`.
- Нужен ли отдельный лимит на ИИ-запросы или общая квота — это сделано
  по твоему ответу «общий ключ, у пользователей лимиты», но если лимит
  должен быть отдельным от анализа — скажи, поправлю.
- DaData — платный сервис выше бесплатного лимита запросов; если не
  подходит, есть альтернативы (ЕГРЮЛ/ФНС API, Контур.Фокус).

## Как накатить

```bash
cd /home/studytool/dev/Geonetix
git checkout -b feat/shared-ai-key-and-org-search

# скопировать новые файлы
cp /path/to/pack/src/*.rs src/
cp /path/to/pack/migrations/0006_app_settings_and_ai.sql migrations/  # переименуй номер при необходимости

# применить точечный патч к Cargo.toml / src/main.rs / .env.example
git apply /path/to/pack/geonetix-patch.diff   # может потребовать ручного мержа — main.rs уже не 158 строк

cargo check
```

`main.rs` в твоей рабочей копии почти наверняка отличается от той, что в
GitHub (роуты модулей 8–12 и т.д.), поэтому патч на `src/main.rs`, скорее
всего, не применится автоматически чисто — тебе нужно руками добавить:
- `mod admin; mod ai_assist; mod claude_client; mod organizations; mod settings;`
- 4 новых роута (см. `geonetix-patch.diff`, секция main.rs).
