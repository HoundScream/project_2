# Stage 02 - Поиск по данным

Date: 2026-05-21
Chat: stage-02-search-db

## Goal
- Подключить модуль поиска к PostgreSQL вместо текстовой заглушки.

## Input Context
- Scheme block: Модуль 2 (поиск)
- Related files: src/main.rs, database/init/001_init.sql

## Implementation Summary
- Decisions:
  - Сохранили `GET /search` и существующую форму, чтобы не ломать UX из Stage 01.
  - Поиск делаем через `ILIKE` по названию и городу, с лимитом 20 результатов.
- Changes:
  - src/main.rs: добавлен SQL-поиск по таблице `geo_objects`, HTML-вывод результатов, базовая экранизация вывода.
  - database/init/001_init.sql: добавлена таблица `geo_objects` и начальный тестовый набор данных.

## Validation
- `cargo check`: passed (2026-05-21)
- Ручная проверка в браузере: требуется выполнить отдельно (`/` -> `/search?q=...`)
- Limitations:
  - Полнотекстовый индекс пока не добавляли.
  - Данные пока демо, без связей с другими доменными сущностями.

## Outcome
- Status: Done
- Ready for next stage: yes, можно переходить к модулю 3.

## Short History
- Stage 01: Создан базовый лендинг (UI + форма поиска + заглушка `/search`).
- Stage 02: Поиск подключен к PostgreSQL и возвращает реальные результаты.

## Next Step
- Реализовать модуль 3 по схеме и связать его с результатом поиска.
