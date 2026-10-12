# Geonetix: структура страниц и переходов

Дата фиксации: 2026-05-27  
База: `http://127.0.0.1:8080`

## 1) Основная пользовательская ветка (публичные страницы)

```mermaid
flowchart TD
  A["/ (Лендинг)"] -->|Форма GET /search| B["/search?q=... (Демо-поиск)"]
  A -->|Кнопка| C["/register (Регистрация)"]
  A -->|Ссылка| E["/pricing (Оплата)"]

  B -->|Ссылка| A
  B -->|Ссылка| C

  C -->|POST /register| D["/register/success/{user_id}"]
  C -->|Ссылка| F["/login (Вход)"]
  C -->|Ссылка| B
  C -->|Ссылка| A

  F -->|POST /login| G["/cabinet/{user_id} (Кабинет)"]
  F -->|Ссылка| C
  F -->|Ссылка| A

  D -->|Кнопка| G
  D -->|Ссылка| H["/cabinet/{user_id}/subscription"]
  D -->|Ссылка| A

  G -->|POST создать анализ| I["/cabinet/{user_id}/analysis/{request_id}"]
  G -->|Ссылка| H
  G -->|Logout| J["/logout -> /"]

  E -->|CTA| K["/pricing/pro"]
```

## 2) PRO-ветка модулей (из роутера)

```mermaid
flowchart TD
  K["/pricing/pro"] -->|POST| M7["/pricing/pro/monitor/{scenario_id}"]
  M7 --> M71["/notifications (модуль 7.1)"]
  M7 --> M8["/module-8"]
  M7 --> M9["/module-9"]
  M71 --> M8
  M71 --> M9
  M8 -->|POST| M8R["/module-8/{audit_id}"]
  M8 --> M10["/module-10"]
  M8 --> M9
  M9 --> M9R["/module-9/{report_id}"]
  M9R --> M9D["/module-9/{report_id}/download"]
  M9R --> M10
  M10 --> M11["/module-11"]
  M10 --> M12["/module-12 (GET/POST)"]
```

## 3) Порядок блоков на лендинге `/`

1. Hero-блок: заголовок + форма `GET /search` + CTA на `/register`.
2. Блок "Что уже заложено в продукт".
3. Блок "Частые вопросы".
4. Финальный CTA-блок: ссылки на `/search`, `/register`, `/pricing`.

## 4) Локальные визуальные артефакты

- Сырые HTML-снимки страниц сохранены в `artifacts/site-snapshots/`:
  - `01-home.html`
  - `02-search-demo.html`
  - `03-register.html`
  - `04-login.html`
  - `05-pricing.html`

## 5) PNG-скриншоты (desktop + mobile)

Файлы сохранены в `artifacts/site-screenshots/`:

- `01-home-desktop.png`
- `01-home-mobile.png`
- `02-search-demo-desktop.png`
- `02-search-demo-mobile.png`
- `03-register-desktop.png`
- `03-register-mobile.png`
- `04-login-desktop.png`
- `04-login-mobile.png`
- `05-pricing-desktop.png`
- `05-pricing-mobile.png`

## 6) Быстрая логика "что за чем идёт"

1. `/` (лендинг): Hero + CTA -> ведёт в демо-поиск (`/search`) или регистрацию (`/register`), также есть переход в тарифы (`/pricing`).
2. `/search`: показывает демо-результаты -> ключевой переход в регистрацию (`/register`).
3. `/register`: создание аккаунта (POST) -> `/register/success/{user_id}`.
4. `/register/success/{user_id}`: точка входа в кабинет -> `/cabinet/{user_id}` и в подписку.
5. `/login`: альтернативный вход в тот же кабинет.
6. `/pricing` -> `/pricing/pro`: запуск платной ветки с мониторингом и модулями 7.1/8/9/10/11/12.
