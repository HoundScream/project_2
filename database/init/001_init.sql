-- Schema is managed by sqlx migrations in migrations/ (run automatically at app startup).
-- This file only bootstraps what the Postgres container needs before the app connects.
create extension if not exists "uuid-ossp";
