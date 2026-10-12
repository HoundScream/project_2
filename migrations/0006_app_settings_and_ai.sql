-- Shared, admin-managed settings (the Claude API key lives here, not per-user:
-- the team decided on a single shared key with per-user quota limits rather
-- than bring-your-own-key).
create table if not exists app_settings (
    key text primary key,
    value text not null,
    updated_at timestamptz not null default now()
);

-- Lightweight audit trail so "who changed the Claude key and when" is
-- answerable without digging through server logs.
create table if not exists app_settings_audit (
    id uuid primary key default uuid_generate_v4(),
    setting_key text not null,
    changed_at timestamptz not null default now(),
    changed_by text
);

-- Track AI-assisted calls against the same per-user quota used for analysis
-- requests, so "AI concierge" usage shows up in one place in the cabinet.
create table if not exists ai_assist_requests (
    id uuid primary key default uuid_generate_v4(),
    app_user_id uuid not null references app_users(id) on delete cascade,
    business_profile_id uuid references business_profiles(id) on delete set null,
    prompt text not null,
    response text,
    model text,
    status text not null default 'queued',
    error_message text,
    created_at timestamptz not null default now(),
    completed_at timestamptz,
    constraint ai_assist_requests_status_check
        check (status in ('queued', 'completed', 'failed'))
);

create index if not exists ai_assist_requests_user_idx
    on ai_assist_requests (app_user_id, created_at desc);
