-- "free_analysis_*" is misleading: the same counter is reused for paid plans too
-- (activate_paid_subscription overwrites the limit and resets the usage on upgrade).
alter table app_users
    rename column free_analysis_limit to analysis_quota_limit;

alter table app_users
    rename column free_analysis_used to analysis_quota_used;

alter table app_users
    rename constraint app_users_free_analysis_limit_check to app_users_analysis_quota_limit_check;

alter table app_users
    rename constraint app_users_free_analysis_used_check to app_users_analysis_quota_used_check;

-- Placeholder for the payment gateway integration: when the current quota period ends
-- and usage should reset. Unused until the gateway work wires it up.
alter table app_users
    add column if not exists analysis_quota_resets_at timestamptz;
