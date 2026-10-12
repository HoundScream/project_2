create extension if not exists "uuid-ossp";

create table if not exists app_healthcheck (
    id uuid primary key default uuid_generate_v4(),
    checked_at timestamptz not null default now(),
    status text not null
);

insert into app_healthcheck (status) values ('ok');

create table if not exists geo_objects (
    id uuid primary key default uuid_generate_v4(),
    name text not null,
    object_type text not null,
    city text not null,
    created_at timestamptz not null default now()
);

insert into geo_objects (name, object_type, city)
values
    ('Смотровая Башня Север', 'Башня', 'Москва'),
    ('Технопарк Восток', 'Технопарк', 'Казань'),
    ('Речной Узел Центр', 'Логистический узел', 'Самара'),
    ('ГеоЛаб Невский', 'Лаборатория', 'Санкт-Петербург')
on conflict do nothing;

create table if not exists app_users (
    id uuid primary key default uuid_generate_v4(),
    full_name text not null,
    phone text not null unique,
    telegram_handle text not null unique,
    email text not null unique,
    password_hash text,
    account_tier text not null default 'free',
    account_status text not null default 'active',
    subscription_plan text not null default 'free',
    subscription_expires_at timestamptz,
    subscription_auto_renew boolean not null default false,
    subscription_cancel_at_period_end boolean not null default false,
    free_analysis_limit integer not null default 1,
    free_analysis_used integer not null default 0,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint app_users_account_tier_check
        check (account_tier in ('free', 'pro')),
    constraint app_users_account_status_check
        check (account_status in ('active', 'blocked')),
    constraint app_users_free_analysis_limit_check
        check (free_analysis_limit >= 0),
    constraint app_users_free_analysis_used_check
        check (free_analysis_used >= 0 and free_analysis_used <= free_analysis_limit)
);

alter table app_users
    add column if not exists password_hash text,
    add column if not exists subscription_plan text not null default 'free',
    add column if not exists subscription_expires_at timestamptz,
    add column if not exists subscription_auto_renew boolean not null default false,
    add column if not exists subscription_cancel_at_period_end boolean not null default false;

do $$
begin
    if not exists (
        select 1
        from pg_constraint
        where conname = 'app_users_subscription_plan_check'
    ) then
        alter table app_users
            add constraint app_users_subscription_plan_check
            check (subscription_plan in ('free', 'pro_3', 'pro_10'));
    end if;
end
$$;

update app_users
set subscription_plan = case
    when account_tier = 'pro' then 'pro_3'
    else 'free'
end
where subscription_plan = 'free'
  and account_tier = 'pro';

create table if not exists app_sessions (
    id uuid primary key default uuid_generate_v4(),
    app_user_id uuid not null references app_users(id) on delete cascade,
    session_token text not null unique,
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now()
);

create table if not exists business_profiles (
    id uuid primary key default uuid_generate_v4(),
    app_user_id uuid not null references app_users(id) on delete cascade,
    source_platform text not null,
    source_url text not null,
    canonical_url text not null,
    profile_title text,
    profile_address text,
    ingestion_status text not null default 'submitted',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint business_profiles_source_platform_check
        check (source_platform in ('yandex_maps', 'google_maps', 'two_gis')),
    constraint business_profiles_ingestion_status_check
        check (ingestion_status in ('submitted', 'normalized', 'ready', 'failed')),
    constraint business_profiles_user_canonical_key
        unique (app_user_id, canonical_url)
);

create table if not exists analysis_requests (
    id uuid primary key default uuid_generate_v4(),
    app_user_id uuid not null references app_users(id) on delete cascade,
    business_profile_id uuid not null references business_profiles(id) on delete cascade,
    request_status text not null default 'queued',
    current_stage text,
    stage_index integer,
    total_stages integer not null default 6,
    progress_percent integer,
    status_message text,
    status_detail text,
    error_message text,
    analysis_kind text not null default 'profile_audit',
    request_source text not null default 'user_cabinet',
    requested_at timestamptz not null default now(),
    completed_at timestamptz,
    created_at timestamptz not null default now(),
    constraint analysis_requests_request_status_check
        check (request_status in ('queued', 'processing', 'completed', 'failed')),
    constraint analysis_requests_analysis_kind_check
        check (analysis_kind in ('profile_audit')),
    constraint analysis_requests_request_source_check
        check (request_source in ('user_cabinet', 'admin', 'api'))
);

alter table analysis_requests
    add column if not exists current_stage text,
    add column if not exists stage_index integer,
    add column if not exists total_stages integer not null default 6,
    add column if not exists progress_percent integer,
    add column if not exists status_message text,
    add column if not exists status_detail text,
    add column if not exists error_message text;

create table if not exists analysis_reports (
    id uuid primary key default uuid_generate_v4(),
    analysis_request_id uuid not null unique references analysis_requests(id) on delete cascade,
    summary text not null,
    strengths text not null,
    risks text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists pro_monitoring_scenarios (
    id uuid primary key default uuid_generate_v4(),
    business_query text not null,
    business_category text not null,
    monitoring_status text not null default 'active',
    check_frequency text not null default 'daily',
    report_format text not null default 'pdf',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pro_monitoring_scenarios_monitoring_status_check
        check (monitoring_status in ('active', 'paused')),
    constraint pro_monitoring_scenarios_check_frequency_check
        check (check_frequency in ('daily')),
    constraint pro_monitoring_scenarios_report_format_check
        check (report_format in ('pdf'))
);

create table if not exists pro_monitoring_events (
    id uuid primary key default uuid_generate_v4(),
    scenario_id uuid not null references pro_monitoring_scenarios(id) on delete cascade,
    competitor_name text not null,
    changed_field text not null,
    change_summary text not null,
    recommendation text not null,
    changed_at timestamptz not null default now(),
    created_at timestamptz not null default now()
);

create table if not exists module_eight_audits (
    id uuid primary key default uuid_generate_v4(),
    scenario_id uuid not null references pro_monitoring_scenarios(id) on delete cascade,
    source_platform text not null,
    source_url text not null,
    canonical_url text not null,
    request_status text not null default 'queued',
    current_stage text,
    progress_percent integer,
    status_message text,
    error_summary text,
    card_title text,
    completeness_score integer,
    photo_count integer,
    menu_items_count integer,
    contact_count integer,
    social_count integer,
    feature_count integer,
    positive_review_share integer,
    negative_review_share integer,
    website_url text,
    social_links text,
    has_delivery boolean not null default false,
    ai_recommendations text,
    competitor_snapshot text,
    request_diagnostics text,
    requested_at timestamptz not null default now(),
    completed_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint module_eight_audits_request_status_check
        check (request_status in ('queued', 'processing', 'completed', 'failed')),
    constraint module_eight_audits_source_platform_check
        check (source_platform in ('yandex_maps', 'google_maps'))
);

alter table module_eight_audits
    add column if not exists website_url text,
    add column if not exists social_links text,
    add column if not exists has_delivery boolean not null default false;

create table if not exists module_nine_reports (
    id uuid primary key default uuid_generate_v4(),
    scenario_id uuid not null references pro_monitoring_scenarios(id) on delete cascade,
    audit_id uuid not null unique references module_eight_audits(id) on delete cascade,
    business_name text not null,
    source_platform text not null,
    card_rating_label text not null,
    seo_score integer not null,
    competitors_summary text not null,
    recommendations text not null,
    strengths text not null,
    weaknesses text not null,
    seo_analysis text not null,
    pdf_status text not null default 'queued',
    pdf_file_name text,
    pdf_disk_path text,
    pdf_generated_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint module_nine_reports_pdf_status_check
        check (pdf_status in ('queued', 'processing', 'ready', 'failed')),
    constraint module_nine_reports_source_platform_check
        check (source_platform in ('yandex_maps', 'google_maps'))
);
