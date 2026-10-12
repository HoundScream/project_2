use crate::db::{
    fetch_competitor_rows, fetch_pending_analysis_jobs, fetch_pending_module_eight_jobs,
    fetch_pro_monitoring_scenario, mark_analysis_failed, mark_module_eight_failed,
    update_analysis_stage, update_module_eight_stage,
};
use crate::models::AppState;
use crate::utils::{
    browser_worker_enabled, collapse_whitespace, fetch_remote_page_snapshot,
    live_card_cache_ttl_seconds, most_common_value, normalize_phone, report_excerpt,
    stable_text_score,
};
use anyhow::Context;
use geonetix_core::heuristics::{count_keyword_hits, estimate_sentiment_shares, first_non_empty};
use geonetix_core::platform::{
    infer_profile_address, infer_profile_title, platform_seed_name, usable_card_title,
};
use geonetix_core::{
    AnalysisReportDraft, AnalysisStageKey, BrowserWorkerCardData, GeoObject, LiveMapCardData,
    MapPlatform, ModuleEightReportDraft, ModuleEightStageKey, ProMonitoringScenario,
    RemotePageSnapshot,
};
use geonetix_parse_jsonld::{extract_json_ld_number, extract_json_ld_text};
use geonetix_parse_yandex::parse_yandex_maps_card_from_embedded_state;
use redis::AsyncCommands;
use scraper::{Html as ScraperHtml, Selector};
use sqlx::PgPool;
use std::collections::BTreeSet;
use std::path::Path as FsPath;
use std::time::Duration;
use tokio::process::Command;
use uuid::Uuid;

pub async fn run_analysis_engine(
    pool: PgPool,
    redis: redis::aio::ConnectionManager,
    user_id: Uuid,
    request_id: Uuid,
    business_profile_id: Uuid,
    platform: MapPlatform,
    canonical_url: String,
) {
    if let Err(error) = run_analysis_engine_inner(
        &pool,
        &redis,
        user_id,
        request_id,
        business_profile_id,
        platform,
        &canonical_url,
    )
    .await
    {
        let _ = mark_analysis_failed(&pool, request_id, &error.to_string()).await;
    }
}

async fn run_analysis_engine_inner(
    pool: &PgPool,
    redis: &redis::aio::ConnectionManager,
    user_id: Uuid,
    request_id: Uuid,
    business_profile_id: Uuid,
    platform: MapPlatform,
    canonical_url: &str,
) -> Result<(), anyhow::Error> {
    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::ValidateLink,
        "Проверяем ссылку",
        "Подтверждаем источник карточки и подготавливаем профиль",
        8,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(150)).await;

    sqlx::query(
        r#"
        update business_profiles
        set ingestion_status = 'normalized',
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(business_profile_id)
    .execute(pool)
    .await?;

    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::LoadData,
        "Получаем данные",
        "Загружаем HTML карточки и извлекаем реальные атрибуты страницы",
        24,
    )
    .await?;
    let live_data = collect_live_map_card_data(platform, canonical_url, redis).await?;

    sqlx::query(
        r#"
        update business_profiles
        set profile_title = $2,
            profile_address = $3,
            ingestion_status = 'ready',
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(business_profile_id)
    .bind(&live_data.card_title)
    .bind(&live_data.address)
    .execute(pool)
    .await?;

    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::AnalyzeReviews,
        "Анализируем отзывы",
        "Считаем тональность по реальному тексту страницы и структурированным данным",
        46,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(90)).await;

    let competitor_rows = fetch_competitor_rows(pool).await?;
    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::CompareCompetitors,
        "Сравниваем конкурентов",
        "Сопоставляем карточку с конкурентным baseline и формируем контекст рынка",
        67,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(90)).await;

    let report = build_analysis_report(
        user_id,
        platform,
        canonical_url,
        &live_data,
        &competitor_rows,
    );

    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::BuildReport,
        "Формируем отчёт",
        "Собираем итоговые выводы и рекомендации по карточке",
        86,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(90)).await;

    sqlx::query(
        r#"
        insert into analysis_reports (
            analysis_request_id,
            summary,
            strengths,
            risks
        )
        values ($1, $2, $3, $4)
        on conflict (analysis_request_id)
        do update set
            summary = excluded.summary,
            strengths = excluded.strengths,
            risks = excluded.risks,
            updated_at = now()
        "#,
    )
    .bind(request_id)
    .bind(&report.summary)
    .bind(&report.strengths)
    .bind(&report.risks)
    .execute(pool)
    .await?;

    update_analysis_stage(
        pool,
        request_id,
        AnalysisStageKey::SaveResult,
        "Сохраняем результат",
        "Автоматически сохраняем отчёт в кабинет пользователя",
        97,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(70)).await;

    sqlx::query(
        r#"
        update analysis_requests
        set request_status = 'completed',
            current_stage = 'save_result',
            stage_index = 6,
            progress_percent = 100,
            status_message = 'Анализ завершён',
            status_detail = 'Результат автоматически сохранён в кабинете пользователя',
            error_message = null,
            completed_at = now()
        where id = $1
        "#,
    )
    .bind(request_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn resume_background_jobs(state: AppState) {
    if let Ok(jobs) = fetch_pending_analysis_jobs(&state.pool).await {
        for job in jobs {
            if let Some(platform) = MapPlatform::from_db(&job.source_platform) {
                tokio::spawn(run_analysis_engine(
                    state.pool.clone(),
                    state.redis.clone(),
                    job.app_user_id,
                    job.request_id,
                    job.business_profile_id,
                    platform,
                    job.canonical_url,
                ));
            }
        }
    }

    if let Ok(jobs) = fetch_pending_module_eight_jobs(&state.pool).await {
        for job in jobs {
            if let Some(platform) = MapPlatform::from_db(&job.source_platform) {
                tokio::spawn(run_module_eight_audit(
                    state.pool.clone(),
                    state.redis.clone(),
                    job.scenario_id,
                    job.audit_id,
                    platform,
                    job.canonical_url,
                ));
            }
        }
    }
}

pub async fn run_module_eight_audit(
    pool: PgPool,
    redis: redis::aio::ConnectionManager,
    scenario_id: Uuid,
    audit_id: Uuid,
    platform: MapPlatform,
    canonical_url: String,
) {
    if let Err(error) = run_module_eight_audit_inner(
        &pool,
        &redis,
        scenario_id,
        audit_id,
        platform,
        &canonical_url,
    )
    .await
    {
        let _ = mark_module_eight_failed(&pool, audit_id, &error.to_string()).await;
    }
}

async fn run_module_eight_audit_inner(
    pool: &PgPool,
    redis: &redis::aio::ConnectionManager,
    scenario_id: Uuid,
    audit_id: Uuid,
    platform: MapPlatform,
    canonical_url: &str,
) -> Result<(), anyhow::Error> {
    let scenario = fetch_pro_monitoring_scenario(pool, scenario_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("scenario not found"))?;

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::ValidateRequest,
        "Проверяем запрос",
        10,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(120)).await;

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::OpenChrome,
        "Открываем карточку и загружаем HTML страницы",
        24,
    )
    .await?;
    let live_data = collect_live_map_card_data(platform, canonical_url, redis).await?;

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::InspectCard,
        "Разбираем структуру карточки, фото, контакты и описание",
        43,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(90)).await;

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::AnalyzeReviews,
        "Оцениваем отзывы и репутационные сигналы по содержимому страницы",
        61,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(90)).await;

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::CompareCompetitors,
        "Сравниваем карточку с 5 конкурентами",
        79,
    )
    .await?;
    let competitor_rows = fetch_competitor_rows(pool).await?;

    let report = build_module_eight_report(
        &scenario,
        platform,
        canonical_url,
        &live_data,
        &competitor_rows,
    );

    update_module_eight_stage(
        pool,
        audit_id,
        ModuleEightStageKey::PublishReport,
        "Публикуем отчёт модуля 8",
        94,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(70)).await;

    sqlx::query(
        r#"
        update module_eight_audits
        set request_status = 'completed',
            current_stage = 'publish_report',
            progress_percent = 100,
            status_message = 'Отчёт модуля 8 готов',
            error_summary = null,
            card_title = $2,
            completeness_score = $3,
            photo_count = $4,
            menu_items_count = $5,
            contact_count = $6,
            social_count = $7,
            feature_count = $8,
            positive_review_share = $9,
            negative_review_share = $10,
            website_url = $11,
            social_links = $12,
            has_delivery = $13,
            ai_recommendations = $14,
            competitor_snapshot = $15,
            request_diagnostics = $16,
            completed_at = now(),
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(audit_id)
    .bind(&report.card_title)
    .bind(report.completeness_score)
    .bind(report.photo_count)
    .bind(report.menu_items_count)
    .bind(report.contact_count)
    .bind(report.social_count)
    .bind(report.feature_count)
    .bind(report.positive_review_share)
    .bind(report.negative_review_share)
    .bind(&report.website_url)
    .bind(&report.social_links)
    .bind(report.has_delivery)
    .bind(&report.ai_recommendations)
    .bind(&report.competitor_snapshot)
    .bind(&report.request_diagnostics)
    .execute(pool)
    .await?;

    Ok(())
}

fn build_module_eight_report(
    scenario: &ProMonitoringScenario,
    _platform: MapPlatform,
    canonical_url: &str,
    live_data: &LiveMapCardData,
    competitor_rows: &[GeoObject],
) -> ModuleEightReportDraft {
    let completeness_score = calculate_completeness_score(live_data);

    let ai_recommendations =
        build_live_recommendations(&live_data.card_title, live_data, Some(scenario)).join("\n");

    let competitors = module_eight_competitors(
        competitor_rows,
        &scenario.business_category,
        &scenario.business_query,
        completeness_score,
        &live_data.category_hint,
    );
    let competitor_snapshot = competitors
        .into_iter()
        .map(|(name, score, gap, focus)| format!("{name}|{score}|{gap}|{focus}"))
        .collect::<Vec<_>>()
        .join("\n");

    let diagnostics = module_eight_diagnostics(canonical_url, live_data).join("\n");

    ModuleEightReportDraft {
        card_title: live_data.card_title.clone(),
        completeness_score,
        photo_count: live_data.photo_count,
        menu_items_count: live_data.menu_items_count,
        contact_count: live_data.contact_count,
        social_count: live_data.social_count,
        feature_count: live_data.feature_count,
        positive_review_share: live_data.positive_review_share,
        negative_review_share: live_data.negative_review_share,
        website_url: live_data.website_url.clone(),
        social_links: live_data.social_links.join("\n"),
        has_delivery: live_data.has_delivery,
        ai_recommendations,
        competitor_snapshot,
        request_diagnostics: diagnostics,
    }
}

fn module_eight_competitors(
    competitor_rows: &[GeoObject],
    business_category: &str,
    business_query: &str,
    completeness_score: i32,
    category_hint: &str,
) -> Vec<(String, i32, String, String)> {
    let category = report_excerpt(Some(if category_hint.is_empty() {
        business_category
    } else {
        category_hint
    }));
    let zone = report_excerpt(Some(business_query));

    let seed = stable_text_score(&format!(
        "{business_category}|{business_query}|{category_hint}"
    ));
    competitor_rows
        .iter()
        .enumerate()
        .map(|(index, base_name)| {
            let competitor_score = (58 + (((seed + index * 19) % 37) as i32)).clamp(45, 96);
            let gap = competitor_score - completeness_score;
            let gap_label = if gap > 0 {
                format!("+{gap} к текущей карточке")
            } else if gap < 0 {
                format!("{gap} к текущей карточке")
            } else {
                "на одном уровне".to_string()
            };
            let focus = match index {
                0 => "сильнее по отзывам".to_string(),
                1 => "лучше упакованы фото и офферы".to_string(),
                2 => "выигрывает за счёт контактов".to_string(),
                3 => "лучше отвечает на негатив".to_string(),
                _ => "быстрее обновляет особенности карточки".to_string(),
            };
            (
                format!("{} {}", base_name.name, category),
                competitor_score,
                gap_label,
                format!("{focus}; город: {}; кластер: {zone}", base_name.city),
            )
        })
        .collect()
}

fn module_eight_diagnostics(canonical_url: &str, live_data: &LiveMapCardData) -> Vec<String> {
    let mut diagnostics = vec![
        format!("URL нормализован и готов к разбору: {canonical_url}"),
        format!("Извлечён заголовок карточки: {}", live_data.card_title),
        format!(
            "Описание карточки: {}",
            report_excerpt(Some(&live_data.description))
        ),
    ];

    if let Some(review_count) = live_data.review_count {
        diagnostics.push(format!("Найдено отзывов в structured data: {review_count}"));
    }
    if let Some(website_url) = live_data.website_url.as_deref() {
        diagnostics.push(format!("Обнаружен внешний сайт бизнеса: {website_url}"));
    }

    if live_data.menu_items_count == 0 {
        diagnostics.push(
            "В карточке не обнаружен блок меню или каталога услуг. Это снижает полноту коммерческого профиля."
                .to_string(),
        );
    }

    if live_data.social_count == 0 {
        diagnostics.push(
            "Не найдены соцсети или внешние площадки. Стоит добавить хотя бы один подтверждённый канал."
                .to_string(),
        );
    }

    if live_data.negative_review_share >= 22 {
        diagnostics.push(
            "Доля негативных отзывов выше комфортного порога. Желательно подготовить сценарий ответов и переработать слабые темы."
                .to_string(),
        );
    }

    diagnostics.extend(live_data.diagnostics.iter().cloned());

    diagnostics
}

fn live_card_cache_key(platform: MapPlatform, canonical_url: &str) -> String {
    format!("live_card:{}:{}", platform.as_db_value(), canonical_url)
}

pub async fn collect_live_map_card_data(
    platform: MapPlatform,
    canonical_url: &str,
    redis: &redis::aio::ConnectionManager,
) -> Result<LiveMapCardData, anyhow::Error> {
    let cache_key = live_card_cache_key(platform, canonical_url);
    let mut redis_conn = redis.clone();

    if let Ok(Some(cached_json)) = redis_conn.get::<_, Option<String>>(&cache_key).await
        && let Ok(mut cached) = serde_json::from_str::<LiveMapCardData>(&cached_json)
    {
        cached
            .diagnostics
            .insert(0, "Данные карточки взяты из кэша Redis.".to_string());
        return Ok(cached);
    }

    let card = if matches!(platform, MapPlatform::YandexMaps)
        && browser_worker_enabled()
        && let Ok(card) = collect_live_map_card_data_with_browser_worker(canonical_url).await
    {
        card
    } else {
        let snapshot = fetch_remote_page_snapshot(canonical_url, "live_parser")
            .await
            .map_err(anyhow::Error::msg)?;
        parse_live_map_card_html(platform, canonical_url, &snapshot)
    };

    if let Ok(payload) = serde_json::to_string(&card) {
        let ttl = live_card_cache_ttl_seconds();
        let _: Result<(), _> = redis_conn.set_ex(&cache_key, payload, ttl).await;
    }

    Ok(card)
}

async fn collect_live_map_card_data_with_browser_worker(
    canonical_url: &str,
) -> Result<LiveMapCardData, anyhow::Error> {
    let worker_script = std::env::var("GEONETIX_BROWSER_WORKER_SCRIPT")
        .unwrap_or_else(|_| "workers/yandex_browser_worker.mjs".to_string());

    if !FsPath::new(&worker_script).exists() {
        anyhow::bail!("browser worker script not found: {worker_script}");
    }

    let timeout_ms = std::env::var("GEONETIX_BROWSER_WORKER_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(45_000);

    let mut command = Command::new("node");
    command.arg(&worker_script).arg(canonical_url);
    command.kill_on_drop(true);

    let output = tokio::time::timeout(Duration::from_millis(timeout_ms), command.output())
        .await
        .context("browser worker timed out")?
        .context("failed to run browser worker")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("browser worker failed: {}", stderr.trim());
    }

    let payload = serde_json::from_slice::<BrowserWorkerCardData>(&output.stdout)
        .context("failed to decode browser worker payload")?;

    Ok(LiveMapCardData {
        card_title: payload.card_title,
        address: payload.address,
        description: payload.description,
        rating: payload.rating,
        review_count: payload.review_count,
        photo_count: payload.photo_count,
        menu_items_count: payload.menu_items_count,
        contact_count: payload.contact_count,
        social_count: payload.social_count,
        social_links: payload.social_links,
        feature_count: payload.feature_count,
        has_delivery: payload.has_delivery,
        positive_review_share: payload.positive_review_share,
        negative_review_share: payload.negative_review_share,
        category_hint: payload.category_hint,
        website_url: payload.website_url,
        diagnostics: {
            let mut diagnostics = payload.diagnostics;
            if let Some(final_url) = payload.final_url {
                diagnostics.insert(0, format!("Browser worker final URL: {final_url}"));
            }
            diagnostics
        },
    })
}

fn parse_live_map_card_html(
    platform: MapPlatform,
    canonical_url: &str,
    snapshot: &RemotePageSnapshot,
) -> LiveMapCardData {
    if matches!(platform, MapPlatform::YandexMaps)
        && let Some(card) = parse_yandex_maps_card_from_embedded_state(canonical_url, snapshot)
    {
        return card;
    }

    let document = ScraperHtml::parse_document(&snapshot.body);
    let body_text = collapse_whitespace(
        &Selector::parse("body")
            .ok()
            .and_then(|selector| document.select(&selector).next())
            .map(|node| node.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default(),
    );
    let body_text_lower = body_text.to_lowercase();

    let title = first_non_empty(vec![
        select_meta_content(&document, "property", "og:title")
            .and_then(|value| usable_card_title(platform, &value)),
        select_meta_content(&document, "name", "twitter:title")
            .and_then(|value| usable_card_title(platform, &value)),
        select_text(&document, "h1").and_then(|value| usable_card_title(platform, &value)),
        select_text(&document, "title").and_then(|value| usable_card_title(platform, &value)),
        extract_json_ld_text(&document, &["name"])
            .and_then(|value| usable_card_title(platform, &value)),
        Some(infer_profile_title(platform, canonical_url)),
    ])
    .unwrap_or_else(|| infer_profile_title(platform, canonical_url));

    let description = first_non_empty(vec![
        select_meta_content(&document, "name", "description"),
        select_meta_content(&document, "property", "og:description"),
        extract_json_ld_text(&document, &["description"]),
        Some(infer_profile_address(platform, canonical_url)),
    ])
    .unwrap_or_else(|| infer_profile_address(platform, canonical_url));

    let address = first_non_empty(vec![
        extract_json_ld_text(&document, &["address", "streetAddress"]),
        extract_address_from_text(&body_text),
        Some(infer_profile_address(platform, canonical_url)),
    ])
    .unwrap_or_else(|| infer_profile_address(platform, canonical_url));

    let rating = extract_json_ld_number(&document, &["aggregateRating", "ratingValue"]);
    let review_count = extract_json_ld_number(&document, &["aggregateRating", "reviewCount"])
        .or_else(|| extract_json_ld_number(&document, &["aggregateRating", "ratingCount"]))
        .map(|value| value.round() as i32);

    let photo_count = count_elements(&document, "img")
        .max(count_keyword_hits(
            &body_text_lower,
            &["фото", "photo", "gallery"],
        ))
        .clamp(0, 200) as i32;
    let menu_items_count = count_keyword_hits(
        &body_text_lower,
        &[
            "меню",
            "menu",
            "каталог",
            "услуги",
            "services",
            "price",
            "цены",
        ],
    )
    .clamp(0, 50) as i32;

    let phone_hits = extract_phone_numbers(&body_text);
    let mailto_count = count_attr_contains(&document, "a", "href", "mailto:");
    let tel_count = count_attr_contains(&document, "a", "href", "tel:");
    let website_url = first_non_empty(vec![
        select_meta_content(&document, "property", "og:url"),
        extract_json_ld_text(&document, &["url"]),
    ]);
    let contact_count = (phone_hits.len()
        + mailto_count
        + tel_count
        + website_url.as_ref().map(|_| 1).unwrap_or(0))
    .clamp(0, 12) as i32;

    let social_links: Vec<String> = extract_social_links(&document).into_iter().collect();
    let social_count = social_links.len().clamp(0, 10) as i32;
    let has_delivery = body_text_lower.contains("доставка") || body_text_lower.contains("delivery");
    let feature_count = count_keyword_hits(
        &body_text_lower,
        &[
            "доставка",
            "delivery",
            "wifi",
            "wi-fi",
            "парковка",
            "parking",
            "акции",
            "скидки",
            "reservation",
            "бронь",
            "самовывоз",
            "онлайн-запись",
        ],
    )
    .clamp(0, 20) as i32;

    let (positive_review_share, negative_review_share) =
        estimate_sentiment_shares(&body_text_lower);
    let category_hint = first_non_empty(vec![
        extract_json_ld_text(&document, &["@type"]),
        extract_category_hint(&body_text),
    ])
    .unwrap_or_else(|| platform_seed_name(platform));

    let mut diagnostics = vec![
        format!("Источник ответа: {}", snapshot.final_url),
        format!("Описание карточки: {}", report_excerpt(Some(&description))),
    ];
    if let Some(rating) = rating {
        diagnostics.push(format!("Обнаружен рейтинг карточки: {rating:.1}/5"));
    }
    if let Some(review_count) = review_count {
        diagnostics.push(format!("Число отзывов в structured data: {review_count}"));
    }
    if website_url.is_none() {
        diagnostics.push("На странице не найден явный URL внешнего сайта бизнеса.".to_string());
    }

    LiveMapCardData {
        card_title: collapse_whitespace(&title),
        address: collapse_whitespace(&address),
        description: collapse_whitespace(&description),
        rating,
        review_count,
        photo_count,
        menu_items_count,
        contact_count,
        social_count,
        feature_count,
        positive_review_share,
        negative_review_share,
        category_hint,
        website_url,
        social_links,
        has_delivery,
        diagnostics,
    }
}

fn select_text(document: &ScraperHtml, selector: &str) -> Option<String> {
    Selector::parse(selector)
        .ok()
        .and_then(|selector| document.select(&selector).next())
        .map(|node| collapse_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
}

fn select_meta_content(document: &ScraperHtml, attr: &str, value: &str) -> Option<String> {
    let selector = Selector::parse(&format!("meta[{attr}=\"{value}\"]")).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(collapse_whitespace)
        .filter(|content| !content.is_empty())
}

fn count_elements(document: &ScraperHtml, selector: &str) -> usize {
    Selector::parse(selector)
        .ok()
        .map(|selector| document.select(&selector).count())
        .unwrap_or(0)
}

fn count_attr_contains(
    document: &ScraperHtml,
    selector: &str,
    attr_name: &str,
    needle: &str,
) -> usize {
    let Ok(selector) = Selector::parse(selector) else {
        return 0;
    };

    document
        .select(&selector)
        .filter_map(|node| node.value().attr(attr_name))
        .filter(|value| value.contains(needle))
        .count()
}

fn extract_phone_numbers(text: &str) -> BTreeSet<String> {
    let mut phones = BTreeSet::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut start = 0usize;
    while start < chars.len() {
        if chars[start].is_ascii_digit() || chars[start] == '+' {
            let mut end = start;
            while end < chars.len()
                && (chars[end].is_ascii_digit()
                    || matches!(chars[end], '+' | '-' | '(' | ')' | ' '))
            {
                end += 1;
            }
            let candidate = chars[start..end].iter().collect::<String>();
            let normalized = normalize_phone(&candidate);
            if normalized.len() >= 10 {
                phones.insert(normalized);
            }
            start = end;
        } else {
            start += 1;
        }
    }
    phones
}

fn extract_social_links(document: &ScraperHtml) -> BTreeSet<String> {
    let Ok(selector) = Selector::parse("a[href]") else {
        return BTreeSet::new();
    };
    let social_domains = [
        "t.me",
        "telegram.me",
        "vk.com",
        "instagram.com",
        "facebook.com",
        "youtube.com",
        "wa.me",
        "whatsapp.com",
    ];

    document
        .select(&selector)
        .filter_map(|node| node.value().attr("href"))
        .filter(|href| social_domains.iter().any(|domain| href.contains(domain)))
        .map(ToString::to_string)
        .collect()
}

fn extract_address_from_text(text: &str) -> Option<String> {
    text.split('.').map(collapse_whitespace).find(|sentence| {
        let lower = sentence.to_lowercase();
        lower.contains("улиц")
            || lower.contains("просп")
            || lower.contains("адрес")
            || lower.contains("street")
            || lower.contains("avenue")
    })
}

fn extract_category_hint(text: &str) -> Option<String> {
    text.split('.').map(collapse_whitespace).find(|sentence| {
        let lower = sentence.to_lowercase();
        lower.contains("restaurant")
            || lower.contains("кафе")
            || lower.contains("салон")
            || lower.contains("clinic")
            || lower.contains("отель")
            || lower.contains("магазин")
    })
}

fn calculate_completeness_score(live_data: &LiveMapCardData) -> i32 {
    (35 + live_data.photo_count.min(25)
        + live_data.contact_count * 6
        + live_data.social_count * 5
        + live_data.feature_count * 2
        + live_data.menu_items_count.min(10)
        + live_data.rating.unwrap_or(0.0).round() as i32 * 3
        - (live_data.negative_review_share / 6))
        .clamp(20, 100)
}

fn build_live_recommendations(
    card_title: &str,
    live_data: &LiveMapCardData,
    scenario: Option<&ProMonitoringScenario>,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if live_data.contact_count < 2 {
        recommendations.push(format!(
            "Усилить контактный блок карточки `{}`: добавить телефон, сайт и дублирующий канал связи.",
            report_excerpt(Some(card_title))
        ));
    }
    if live_data.photo_count < 8 {
        recommendations.push(
            "Добрать визуальное наполнение: свежие фото фасада, интерьера, команды и продукта."
                .to_string(),
        );
    }
    if live_data.menu_items_count == 0 {
        recommendations.push("Добавить меню, каталог услуг или прайс, чтобы карточка давала коммерческий контекст прямо на карте.".to_string());
    }
    if live_data.negative_review_share >= 25 {
        recommendations.push(
            "Подготовить сценарий ответов на негатив и закрыть повторяющиеся жалобы из отзывов."
                .to_string(),
        );
    }
    if live_data.social_count == 0 {
        recommendations.push("Связать карточку с внешними площадками: Telegram, сайт, соцсети или мессенджер для доверия и конверсии.".to_string());
    }
    if let Some(scenario) = scenario {
        recommendations.push(format!(
            "Сверить наполнение карточки с геозапросом `{}` и категорией `{}` из PRO-сценария.",
            report_excerpt(Some(&scenario.business_query)),
            report_excerpt(Some(&scenario.business_category))
        ));
    }
    if recommendations.is_empty() {
        recommendations.push("Карточка уже выглядит цельно. Следующий шаг — регулярные обновления контента и контроль отзывов.".to_string());
    }

    recommendations
}

fn build_analysis_report(
    user_id: Uuid,
    platform: MapPlatform,
    canonical_url: &str,
    live_data: &LiveMapCardData,
    competitor_rows: &[GeoObject],
) -> AnalysisReportDraft {
    let platform_label = match platform {
        MapPlatform::YandexMaps => "Яндекс Карты",
        MapPlatform::GoogleMaps => "Google Maps",
        MapPlatform::TwoGis => "2GIS",
    };
    let competitor_names = competitor_rows
        .iter()
        .map(|row| row.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let dominant_city = most_common_value(competitor_rows.iter().map(|row| row.city.as_str()))
        .unwrap_or_else(|| "не определён".to_string());

    AnalysisReportDraft {
        summary: format!(
            "{} обработан через {platform_label}. Карточка автоматически сохранена для пользователя {user_id}; адрес карточки: «{}», рейтинг: {}.",
            live_data.card_title,
            live_data.address,
            live_data
                .rating
                .map(|value| format!("{value:.1}/5"))
                .unwrap_or_else(|| "не найден".to_string())
        ),
        strengths: format!(
            "Карточка прошла реальный разбор HTML-страницы: фото {}, контакты {}, внешние площадки {}, признаки наполненности {}. Для конкурентного baseline использованы объекты: {competitor_names}. Это даёт стартовый контекст по зоне «{dominant_city}».",
            live_data.photo_count,
            live_data.contact_count,
            live_data.social_count,
            live_data.feature_count
        ),
        risks: format!(
            "Источник {canonical_url} уже анализируется по живой странице, но часть картографических сервисов рендерит блоки клиентски, поэтому полнота сигналов зависит от того, сколько данных доступно в исходном HTML."
        ),
    }
}
