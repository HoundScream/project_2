use crate::utils::{
    collapse_whitespace, infer_profile_title, monitoring_event_seed, pdf_escape_text,
    platform_label_from_db, read_cookie_value, report_excerpt, sanitize_file_stem,
    transliterate_to_ascii,
};
use axum::http::HeaderMap;
use geonetix_core::{
    AnalysisStageKey, AnalysisStatusRecord, AuthUserRecord, CabinetAnalysisRecord, GeoObject,
    MapPlatform, ModuleEightAuditRecord, ModuleEightAuditSummary, ModuleEightCompletedAuditRow,
    ModuleEightStageKey, ModuleNineReportDraft, ModuleNineReportRecord, ModuleNineReportSummary,
    PendingAnalysisJob, PendingModuleEightJob, ProMonitoringEvent, ProMonitoringScenario,
    ProPackageForm, RegisteredUser, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS, SubscriptionFeedback,
};
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

pub async fn update_analysis_stage(
    pool: &PgPool,
    request_id: Uuid,
    stage: AnalysisStageKey,
    message: &str,
    detail: &str,
    progress_percent: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        update analysis_requests
        set request_status = 'processing',
            current_stage = $2,
            stage_index = $3,
            total_stages = 6,
            progress_percent = $4,
            status_message = $5,
            status_detail = $6,
            error_message = null
        where id = $1
        "#,
    )
    .bind(request_id)
    .bind(stage.as_db_value())
    .bind(stage.index())
    .bind(progress_percent)
    .bind(message)
    .bind(detail)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_analysis_failed(
    pool: &PgPool,
    request_id: Uuid,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        update analysis_requests
        set request_status = 'failed',
            progress_percent = coalesce(progress_percent, 18),
            status_message = 'Не удалось завершить анализ',
            status_detail = 'Фоновый обработчик остановился с ошибкой',
            error_message = $2,
            completed_at = now()
        where id = $1
        "#,
    )
    .bind(request_id)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn has_duplicate_business_profile(
    pool: &PgPool,
    user_id: Uuid,
    canonical_url: &str,
) -> Result<bool, sqlx::Error> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        select id
        from business_profiles
        where app_user_id = $1
          and canonical_url = $2
        limit 1
        "#,
    )
    .bind(user_id)
    .bind(canonical_url)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub async fn fetch_auth_user(
    pool: &PgPool,
    normalized_email: &str,
    normalized_phone: &str,
) -> Result<Option<AuthUserRecord>, sqlx::Error> {
    sqlx::query_as::<_, AuthUserRecord>(
        r#"
        select id, password_hash
        from app_users
        where email = $1
           or phone = $2
        limit 1
        "#,
    )
    .bind(normalized_email)
    .bind(normalized_phone)
    .fetch_optional(pool)
    .await
}

pub async fn create_user_session(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let token = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        insert into app_sessions (
            app_user_id,
            session_token,
            expires_at
        )
        values ($1, $2, now() + make_interval(secs => $3))
        "#,
    )
    .bind(user_id)
    .bind(&token)
    .bind(SESSION_TTL_SECONDS as i32)
    .execute(pool)
    .await?;
    Ok(token)
}

pub async fn fetch_session_user(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<Option<RegisteredUser>, sqlx::Error> {
    let Some(token) = read_cookie_value(headers, SESSION_COOKIE_NAME) else {
        return Ok(None);
    };

    let user = sqlx::query_as::<_, RegisteredUser>(
        r#"
        select
            u.id,
            u.full_name,
            u.phone,
            u.telegram_handle,
            u.email,
            u.account_tier,
            u.subscription_plan,
            to_char(u.subscription_expires_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as subscription_expires_at_label,
            u.subscription_auto_renew,
            u.subscription_cancel_at_period_end,
            case
                when u.account_tier = 'pro'
                    and (u.subscription_expires_at is null or u.subscription_expires_at >= now())
                then true
                else false
            end as subscription_is_active,
            u.analysis_quota_limit,
            u.analysis_quota_used
        from app_sessions s
        join app_users u on u.id = s.app_user_id
        where s.session_token = $1
          and s.expires_at >= now()
        limit 1
        "#,
    )
    .bind(&token)
    .fetch_optional(pool)
    .await?;

    if user.is_some() {
        let _ = sqlx::query(
            r#"
            update app_sessions
            set last_seen_at = now(),
                expires_at = now() + make_interval(secs => $2)
            where session_token = $1
            "#,
        )
        .bind(&token)
        .bind(SESSION_TTL_SECONDS as i32)
        .execute(pool)
        .await;
    }

    Ok(user)
}

pub async fn activate_paid_subscription(
    pool: &PgPool,
    user_id: Uuid,
    plan: &str,
    limit: i32,
    extend_period: bool,
) -> Result<(), String> {
    sqlx::query(
        r#"
        update app_users
        set account_tier = 'pro',
            subscription_plan = $2,
            subscription_expires_at = case
                when $4 = true then greatest(coalesce(subscription_expires_at, now()), now()) + interval '30 days'
                when subscription_expires_at is null or subscription_expires_at < now() then now() + interval '30 days'
                else subscription_expires_at
            end,
            subscription_auto_renew = true,
            subscription_cancel_at_period_end = false,
            analysis_quota_limit = $3,
            analysis_quota_used = 0,
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(user_id)
    .bind(plan)
    .bind(limit)
    .bind(extend_period)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|_| "Не удалось активировать платный пакет.".to_string())
}

pub async fn fetch_registered_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<RegisteredUser>, sqlx::Error> {
    sqlx::query_as::<_, RegisteredUser>(
        r#"
        select
            id,
            full_name,
            phone,
            telegram_handle,
            email,
            account_tier,
            subscription_plan,
            to_char(subscription_expires_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as subscription_expires_at_label,
            subscription_auto_renew,
            subscription_cancel_at_period_end,
            case
                when account_tier = 'pro'
                    and (subscription_expires_at is null or subscription_expires_at >= now())
                then true
                else false
            end as subscription_is_active,
            analysis_quota_limit,
            analysis_quota_used
        from app_users
        where id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_user_analysis_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CabinetAnalysisRecord>, sqlx::Error> {
    sqlx::query_as::<_, CabinetAnalysisRecord>(
        r#"
        select
            ar.id as request_id,
            ar.request_status,
            ar.analysis_kind,
            to_char(ar.requested_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as requested_at,
            bp.source_platform,
            bp.canonical_url,
            bp.profile_title,
            bp.profile_address
        from analysis_requests ar
        join business_profiles bp on bp.id = ar.business_profile_id
        where ar.app_user_id = $1
        order by ar.requested_at desc
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_analysis_status_record(
    pool: &PgPool,
    user_id: Uuid,
    request_id: Uuid,
) -> Result<Option<AnalysisStatusRecord>, sqlx::Error> {
    sqlx::query_as::<_, AnalysisStatusRecord>(
        r#"
        select
            ar.id as request_id,
            ar.request_status,
            ar.analysis_kind,
            to_char(ar.requested_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as requested_at_label,
            ar.current_stage,
            ar.stage_index,
            ar.total_stages,
            ar.progress_percent,
            ar.status_message,
            ar.status_detail,
            ar.error_message,
            bp.source_platform,
            bp.canonical_url,
            bp.profile_title,
            bp.profile_address,
            rpt.summary as report_summary,
            rpt.strengths as report_strengths,
            rpt.risks as report_risks
        from analysis_requests ar
        join business_profiles bp on bp.id = ar.business_profile_id
        left join analysis_reports rpt on rpt.analysis_request_id = ar.id
        where ar.app_user_id = $1
          and ar.id = $2
        "#,
    )
    .bind(user_id)
    .bind(request_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_pending_analysis_jobs(
    pool: &PgPool,
) -> Result<Vec<PendingAnalysisJob>, sqlx::Error> {
    sqlx::query_as::<_, PendingAnalysisJob>(
        r#"
        select
            ar.id as request_id,
            ar.app_user_id,
            ar.business_profile_id,
            bp.source_platform,
            bp.canonical_url
        from analysis_requests ar
        join business_profiles bp on bp.id = ar.business_profile_id
        where ar.request_status in ('queued', 'processing')
        order by ar.requested_at asc
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_pending_module_eight_jobs(
    pool: &PgPool,
) -> Result<Vec<PendingModuleEightJob>, sqlx::Error> {
    sqlx::query_as::<_, PendingModuleEightJob>(
        r#"
        select
            id as audit_id,
            scenario_id,
            source_platform,
            canonical_url
        from module_eight_audits
        where request_status in ('queued', 'processing')
        order by requested_at asc
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn create_pro_monitoring_scenario(
    pool: &PgPool,
    form: &ProPackageForm,
) -> Result<Uuid, sqlx::Error> {
    let business_query = collapse_whitespace(&form.business_query);
    let business_category = collapse_whitespace(&form.business_category);

    let (scenario_id,) = sqlx::query_as::<_, (Uuid,)>(
        r#"
        insert into pro_monitoring_scenarios (
            business_query,
            business_category
        )
        values ($1, $2)
        returning id
        "#,
    )
    .bind(&business_query)
    .bind(&business_category)
    .fetch_one(pool)
    .await?;

    let seeds = monitoring_event_seed(&business_query, &business_category);
    for (competitor_name, changed_field, change_summary, recommendation) in seeds {
        sqlx::query(
            r#"
            insert into pro_monitoring_events (
                scenario_id,
                competitor_name,
                changed_field,
                change_summary,
                recommendation
            )
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(scenario_id)
        .bind(competitor_name)
        .bind(changed_field)
        .bind(change_summary)
        .bind(recommendation)
        .execute(pool)
        .await?;
    }

    Ok(scenario_id)
}

pub async fn fetch_pro_monitoring_scenario(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Option<ProMonitoringScenario>, sqlx::Error> {
    sqlx::query_as::<_, ProMonitoringScenario>(
        r#"
        select
            id,
            business_query,
            business_category,
            monitoring_status,
            check_frequency,
            report_format,
            to_char(created_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as created_at_label
        from pro_monitoring_scenarios
        where id = $1
        "#,
    )
    .bind(scenario_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_pro_monitoring_events(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ProMonitoringEvent>, sqlx::Error> {
    sqlx::query_as::<_, ProMonitoringEvent>(
        r#"
        select
            to_char(changed_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as changed_at_label,
            competitor_name,
            changed_field,
            change_summary,
            recommendation
        from pro_monitoring_events
        where scenario_id = $1
        order by changed_at desc, created_at desc
        "#,
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_module_eight_audits(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ModuleEightAuditSummary>, sqlx::Error> {
    sqlx::query_as::<_, ModuleEightAuditSummary>(
        r#"
        select
            id as audit_id,
            request_status,
            source_platform,
            to_char(requested_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as requested_at_label,
            card_title,
            status_message,
            completeness_score
        from module_eight_audits
        where scenario_id = $1
        order by requested_at desc, created_at desc
        "#,
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_module_eight_audit(
    pool: &PgPool,
    scenario_id: Uuid,
    audit_id: Uuid,
) -> Result<Option<ModuleEightAuditRecord>, sqlx::Error> {
    sqlx::query_as::<_, ModuleEightAuditRecord>(
        r#"
        select
            source_platform,
            source_url,
            canonical_url,
            request_status,
            current_stage,
            progress_percent,
            status_message,
            error_summary,
            card_title,
            completeness_score,
            photo_count,
            menu_items_count,
            contact_count,
            social_count,
            feature_count,
            positive_review_share,
            negative_review_share,
            website_url,
            social_links,
            has_delivery,
            ai_recommendations,
            competitor_snapshot,
            request_diagnostics,
            to_char(requested_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as requested_at_label
        from module_eight_audits
        where scenario_id = $1
          and id = $2
        "#,
    )
    .bind(scenario_id)
    .bind(audit_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_module_nine_reports(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ModuleNineReportSummary>, sqlx::Error> {
    sqlx::query_as::<_, ModuleNineReportSummary>(
        r#"
        select
            id as report_id,
            to_char(created_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as created_at_label,
            business_name,
            source_platform,
            card_rating_label,
            seo_score,
            pdf_status,
            to_char(pdf_generated_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as pdf_generated_at_label
        from module_nine_reports
        where scenario_id = $1
        order by created_at desc
        "#,
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_module_nine_report(
    pool: &PgPool,
    scenario_id: Uuid,
    report_id: Uuid,
) -> Result<Option<ModuleNineReportRecord>, sqlx::Error> {
    sqlx::query_as::<_, ModuleNineReportRecord>(
        r#"
        select
            id as report_id,
            to_char(created_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as created_at_label,
            business_name,
            source_platform,
            card_rating_label,
            seo_score,
            competitors_summary,
            recommendations,
            strengths,
            weaknesses,
            seo_analysis,
            pdf_status,
            pdf_file_name,
            pdf_disk_path,
            to_char(pdf_generated_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as pdf_generated_at_label
        from module_nine_reports
        where scenario_id = $1
          and id = $2
        "#,
    )
    .bind(scenario_id)
    .bind(report_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_completed_module_eight_rows(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<(Uuid, ModuleEightAuditRecord)>, sqlx::Error> {
    sqlx::query_as::<_, ModuleEightCompletedAuditRow>(
        r#"
        select
            id as audit_id,
            source_platform,
            source_url,
            canonical_url,
            request_status,
            current_stage,
            progress_percent,
            status_message,
            error_summary,
            card_title,
            completeness_score,
            photo_count,
            menu_items_count,
            contact_count,
            social_count,
            feature_count,
            positive_review_share,
            negative_review_share,
            website_url,
            social_links,
            has_delivery,
            ai_recommendations,
            competitor_snapshot,
            request_diagnostics,
            to_char(requested_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as requested_at_label
        from module_eight_audits
        where scenario_id = $1
          and request_status = 'completed'
        order by requested_at desc, created_at desc
        "#,
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|row| {
                (
                    row.audit_id,
                    ModuleEightAuditRecord {
                        source_platform: row.source_platform,
                        source_url: row.source_url,
                        canonical_url: row.canonical_url,
                        request_status: row.request_status,
                        current_stage: row.current_stage,
                        progress_percent: row.progress_percent,
                        status_message: row.status_message,
                        error_summary: row.error_summary,
                        card_title: row.card_title,
                        completeness_score: row.completeness_score,
                        photo_count: row.photo_count,
                        menu_items_count: row.menu_items_count,
                        contact_count: row.contact_count,
                        social_count: row.social_count,
                        feature_count: row.feature_count,
                        positive_review_share: row.positive_review_share,
                        negative_review_share: row.negative_review_share,
                        website_url: row.website_url,
                        social_links: row.social_links,
                        has_delivery: row.has_delivery,
                        ai_recommendations: row.ai_recommendations,
                        competitor_snapshot: row.competitor_snapshot,
                        request_diagnostics: row.request_diagnostics,
                        requested_at_label: row.requested_at_label,
                    },
                )
            })
            .collect()
    })
}

pub async fn sync_module_nine_reports(
    pool: &PgPool,
    report_storage_dir: &Path,
    scenario_id: Uuid,
) -> Result<(), anyhow::Error> {
    let scenario = fetch_pro_monitoring_scenario(pool, scenario_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("scenario not found"))?;
    let audits = fetch_completed_module_eight_rows(pool, scenario_id).await?;

    for (audit_id, audit) in audits {
        let draft = build_module_nine_draft(&scenario, &audit);
        let inserted = sqlx::query_as::<_, (Uuid,)>(
            r#"
            insert into module_nine_reports (
                scenario_id,
                audit_id,
                business_name,
                source_platform,
                card_rating_label,
                seo_score,
                competitors_summary,
                recommendations,
                strengths,
                weaknesses,
                seo_analysis,
                pdf_status
            )
            select
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10,
                $11,
                'queued'
            where not exists (
                select 1 from module_nine_reports existing where existing.audit_id = $2
            )
            returning id
            "#,
        )
        .bind(scenario_id)
        .bind(audit_id)
        .bind(draft.business_name)
        .bind(draft.source_platform)
        .bind(draft.card_rating_label)
        .bind(draft.seo_score)
        .bind(draft.competitors_summary)
        .bind(draft.recommendations)
        .bind(draft.strengths)
        .bind(draft.weaknesses)
        .bind(draft.seo_analysis)
        .fetch_optional(pool)
        .await?;

        if inserted.is_some() {
            generate_module_nine_pdf_for_audit(pool, report_storage_dir, scenario_id, audit_id)
                .await?;
        }
    }

    let queued_reports = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        select id, audit_id
        from module_nine_reports
        where scenario_id = $1
          and pdf_status in ('queued', 'failed')
        order by created_at asc
        "#,
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;

    for (_, audit_id) in queued_reports {
        generate_module_nine_pdf_for_audit(pool, report_storage_dir, scenario_id, audit_id).await?;
    }

    Ok(())
}

pub async fn generate_module_nine_pdf_for_audit(
    pool: &PgPool,
    report_storage_dir: &Path,
    scenario_id: Uuid,
    audit_id: Uuid,
) -> Result<(), anyhow::Error> {
    let report = sqlx::query_as::<_, ModuleNineReportRecord>(
        r#"
        select
            id as report_id,
            to_char(created_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as created_at_label,
            business_name,
            source_platform,
            card_rating_label,
            seo_score,
            competitors_summary,
            recommendations,
            strengths,
            weaknesses,
            seo_analysis,
            pdf_status,
            pdf_file_name,
            pdf_disk_path,
            to_char(pdf_generated_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as pdf_generated_at_label
        from module_nine_reports
        where scenario_id = $1
          and audit_id = $2
        "#,
    )
    .bind(scenario_id)
    .bind(audit_id)
    .fetch_one(pool)
    .await?;

    if report.pdf_status == "ready"
        && let Some(path) = report.pdf_disk_path.as_deref()
        && tokio::fs::metadata(path).await.is_ok()
    {
        return Ok(());
    }

    sqlx::query(
        r#"
        update module_nine_reports
        set pdf_status = 'processing',
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(report.report_id)
    .execute(pool)
    .await?;

    let pdf_bytes = build_module_nine_pdf_bytes(&report);
    let file_name = format!(
        "module9-{}-{}.pdf",
        sanitize_file_stem(&report.business_name),
        report.report_id
    );
    let disk_path = report_storage_dir.join(&file_name);

    match tokio::fs::write(&disk_path, pdf_bytes).await {
        Ok(_) => {
            sqlx::query(
                r#"
                update module_nine_reports
                set pdf_status = 'ready',
                    pdf_file_name = $2,
                    pdf_disk_path = $3,
                    pdf_generated_at = now(),
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(report.report_id)
            .bind(&file_name)
            .bind(disk_path.to_string_lossy().to_string())
            .execute(pool)
            .await?;
        }
        Err(error) => {
            sqlx::query(
                r#"
                update module_nine_reports
                set pdf_status = 'failed',
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(report.report_id)
            .execute(pool)
            .await?;
            return Err(error.into());
        }
    }

    Ok(())
}

pub async fn update_module_eight_stage(
    pool: &PgPool,
    audit_id: Uuid,
    stage: ModuleEightStageKey,
    status_message: &str,
    progress_percent: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        update module_eight_audits
        set request_status = 'processing',
            current_stage = $2,
            progress_percent = $3,
            status_message = $4,
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(audit_id)
    .bind(stage.as_db_value())
    .bind(progress_percent)
    .bind(status_message)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_module_eight_failed(
    pool: &PgPool,
    audit_id: Uuid,
    error_summary: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        update module_eight_audits
        set request_status = 'failed',
            progress_percent = coalesce(progress_percent, 22),
            status_message = 'Модуль 8 завершился с ошибкой',
            error_summary = $2,
            completed_at = now(),
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(audit_id)
    .bind(error_summary)
    .execute(pool)
    .await?;

    Ok(())
}

fn build_module_nine_draft(
    scenario: &ProMonitoringScenario,
    audit: &ModuleEightAuditRecord,
) -> ModuleNineReportDraft {
    let source_platform =
        MapPlatform::from_db(&audit.source_platform).unwrap_or(MapPlatform::YandexMaps);
    let business_name = audit
        .card_title
        .clone()
        .unwrap_or_else(|| infer_profile_title(source_platform, &audit.canonical_url));
    let completeness = audit.completeness_score.unwrap_or(0).clamp(0, 100);
    let rating = 3.4 + (completeness as f32 / 100.0) * 1.6;
    let card_rating_label = format!("{rating:.1}/5");
    let seo_score = (completeness
        + audit.contact_count.unwrap_or(0) * 3
        + audit.social_count.unwrap_or(0) * 4
        + audit.menu_items_count.unwrap_or(0) * 2
        - audit.negative_review_share.unwrap_or(0))
    .clamp(34, 98);

    let competitors_summary = audit
        .competitor_snapshot
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '|');
            Some(format!(
                "{}: индекс {}%, {}, фокус {}.",
                parts.next()?,
                parts.next()?,
                parts.next()?,
                parts.next()?
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let recommendations = audit.ai_recommendations.clone().unwrap_or_else(|| {
        "Добавить больше данных в карточку и усилить ответы на отзывы.".to_string()
    });

    let strengths = format!(
        "Карточка `{}` уже имеет базовую наполненность {}%, а сценарий `{}` даёт локальный контекст по конкурентам и отзывам.",
        report_excerpt(Some(&business_name)),
        completeness,
        report_excerpt(Some(&scenario.business_query))
    );

    let weaknesses = if audit.negative_review_share.unwrap_or(0) >= 20 {
        "Высока доля негативных отзывов, а часть коммерческих полей карточки требует усиления. Это может просаживать локальную видимость и доверие пользователей.".to_string()
    } else {
        "Карточка выглядит стабильной, но ей всё ещё не хватает части коммерческих сигналов: дополнительных фото, точных CTA, внешних ссылок и регулярных обновлений профиля.".to_string()
    };

    let seo_analysis = format!(
        "SEO-профиль строится на источнике `{}`, рейтинге карточки {}, полноте данных {}% и тональности отзывов. Для категории `{}` стоит усилить ключевые коммерческие поля, обновлять контент и держать контактные блоки синхронными на карте и сайте.",
        platform_label_from_db(&audit.source_platform),
        card_rating_label,
        completeness,
        report_excerpt(Some(&scenario.business_category))
    );

    ModuleNineReportDraft {
        business_name,
        source_platform: audit.source_platform.clone(),
        card_rating_label,
        seo_score,
        competitors_summary,
        recommendations,
        strengths,
        weaknesses,
        seo_analysis,
    }
}

pub async fn fetch_competitor_rows(pool: &PgPool) -> Result<Vec<GeoObject>, sqlx::Error> {
    sqlx::query_as::<_, GeoObject>(
        r#"
        select name, object_type, city
        from geo_objects
        order by name asc
        limit 4
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn apply_subscription_action(
    pool: &PgPool,
    user_id: Uuid,
    action: &str,
) -> Result<SubscriptionFeedback, String> {
    let user = fetch_registered_user(pool, user_id)
        .await
        .map_err(|_| "Не удалось загрузить текущую подписку.".to_string())?
        .ok_or_else(|| "Пользователь не найден.".to_string())?;

    match action {
        "renew_pro_3" => activate_paid_subscription(pool, user_id, "pro_3", 3, true)
            .await
            .map(|_| SubscriptionFeedback {
                title: "Лимит продлён".to_string(),
                detail: "Подписка PRO 3 активирована ещё на 30 дней, лимит запусков обновлён."
                    .to_string(),
                is_error: false,
            }),
        "renew_pro_10" => activate_paid_subscription(pool, user_id, "pro_10", 10, true)
            .await
            .map(|_| SubscriptionFeedback {
                title: "Подписка обновлена".to_string(),
                detail: "Пакет PRO 10 активирован, новый лимит запусков уже доступен в кабинете."
                    .to_string(),
                is_error: false,
            }),
        "change_to_pro_3" => activate_paid_subscription(pool, user_id, "pro_3", 3, false)
            .await
            .map(|_| SubscriptionFeedback {
                title: "Тариф изменён".to_string(),
                detail: "Теперь у пользователя пакет PRO 3 с обновлённым лимитом анализов."
                    .to_string(),
                is_error: false,
            }),
        "change_to_pro_10" => activate_paid_subscription(pool, user_id, "pro_10", 10, false)
            .await
            .map(|_| SubscriptionFeedback {
                title: "Тариф изменён".to_string(),
                detail: "Теперь активен пакет PRO 10 с расширенным лимитом анализов.".to_string(),
                is_error: false,
            }),
        "toggle_auto_renew" => {
            if user.account_tier != "pro" {
                return Err(
                    "Автопродление доступно только для активной или недавно завершившейся PRO-подписки."
                        .to_string(),
                );
            }

            sqlx::query(
                r#"
                update app_users
                set subscription_auto_renew = not subscription_auto_renew,
                    subscription_cancel_at_period_end = case
                        when subscription_auto_renew = false then false
                        else subscription_cancel_at_period_end
                    end,
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|_| "Не удалось переключить автопродление.".to_string())?;

            let enabled = !user.subscription_auto_renew;
            Ok(SubscriptionFeedback {
                title: if enabled {
                    "Автопродление включено".to_string()
                } else {
                    "Автопродление выключено".to_string()
                },
                detail: if enabled {
                    "Подписка продолжит продлеваться автоматически, пока вы её не отмените."
                        .to_string()
                } else {
                    "Автосписание отключено. Подписка останется активной до конца оплаченного периода.".to_string()
                },
                is_error: false,
            })
        }
        "cancel_subscription" => {
            if user.account_tier != "pro" {
                return Err("Отменять можно только PRO-подписку.".to_string());
            }

            sqlx::query(
                r#"
                update app_users
                set subscription_auto_renew = false,
                    subscription_cancel_at_period_end = true,
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|_| "Не удалось отменить подписку.".to_string())?;

            Ok(SubscriptionFeedback {
                title: "Подписка отменена".to_string(),
                detail: "Автопродление отключено. Пользователь сохранит доступ до конца уже оплаченного периода.".to_string(),
                is_error: false,
            })
        }
        _ => Err("Неизвестное действие для модуля 13.".to_string()),
    }
}

fn build_module_nine_pdf_bytes(report: &ModuleNineReportRecord) -> Vec<u8> {
    let lines = build_module_nine_pdf_lines(report);
    build_simple_pdf_document(&lines)
}

fn build_module_nine_pdf_lines(report: &ModuleNineReportRecord) -> Vec<String> {
    let sections = [
        "Module 9 Extended Request History".to_string(),
        format!(
            "Business: {}",
            transliterate_to_ascii(&report.business_name)
        ),
        format!("Analysis date: {}", report.created_at_label),
        format!(
            "Map source: {}",
            transliterate_to_ascii(platform_label_from_db(&report.source_platform))
        ),
        format!("Card rating: {}", report.card_rating_label),
        format!("SEO score: {}/100", report.seo_score),
        String::new(),
        "Competitors:".to_string(),
        transliterate_to_ascii(&report.competitors_summary),
        String::new(),
        "Recommendations:".to_string(),
        transliterate_to_ascii(&report.recommendations),
        String::new(),
        "Strengths:".to_string(),
        transliterate_to_ascii(&report.strengths),
        String::new(),
        "Weaknesses:".to_string(),
        transliterate_to_ascii(&report.weaknesses),
        String::new(),
        "SEO analysis:".to_string(),
        transliterate_to_ascii(&report.seo_analysis),
    ];

    sections
        .into_iter()
        .flat_map(|line| wrap_pdf_line(&line, 92))
        .collect()
}

fn build_simple_pdf_document(lines: &[String]) -> Vec<u8> {
    let lines_per_page = 42usize;
    let pages = lines.chunks(lines_per_page).collect::<Vec<_>>();
    let font_object_id = 3 + (pages.len() * 2) as i32;

    let mut objects = Vec::new();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());

    let kids = pages
        .iter()
        .enumerate()
        .map(|(index, _)| format!("{} 0 R", 3 + (index as i32 * 2)))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids,
        pages.len()
    ));

    for (index, page_lines) in pages.iter().enumerate() {
        let page_object_id = 3 + (index as i32 * 2);
        let content_object_id = page_object_id + 1;
        let content = build_pdf_content_stream(page_lines);
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {font_object_id} 0 R >> >> /Contents {content_object_id} 0 R >>"
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ));
    }

    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );

    pdf
}

fn build_pdf_content_stream(lines: &[String]) -> String {
    let mut content = String::from("BT\n/F1 11 Tf\n50 790 Td\n14 TL\n");
    for (index, line) in lines.iter().enumerate() {
        if index == 0 {
            content.push_str(&format!("({}) Tj\n", pdf_escape_text(line)));
        } else {
            content.push_str(&format!("T* ({}) Tj\n", pdf_escape_text(line)));
        }
    }
    content.push_str("ET");
    content
}

fn wrap_pdf_line(value: &str, width: usize) -> Vec<String> {
    if value.trim().is_empty() {
        return vec![String::new()];
    }

    let mut output = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if candidate.len() > width && !current.is_empty() {
            output.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        output.push(current);
    }
    output
}
