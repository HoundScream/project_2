pub use geonetix_core::platform::infer_profile_title;
pub use geonetix_core::text::{collapse_whitespace, report_excerpt};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::http::HeaderMap;
use axum::{
    http::{HeaderValue, header},
    response::{IntoResponse, Redirect, Response},
};
use geonetix_core::{
    GeoObject, MapPlatform, ParsedBusinessUrl, ProPackageForm, QueryAnalysis, RegisterForm,
    RegisteredUser, RemotePageSnapshot, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS, ValidationIssue,
};

use rand_core::OsRng;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use url::Url;

pub fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn normalize_query(value: &str) -> String {
    collapse_whitespace(value).to_lowercase()
}

pub fn most_common_value<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0usize) += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
        .map(|(value, _)| value.to_string())
}

pub fn analyze_query(query: &str, rows: &[GeoObject]) -> QueryAnalysis {
    let normalized_query = normalize_query(query);
    let exact_match_count = rows
        .iter()
        .filter(|row| {
            normalize_query(&row.name).contains(&normalized_query)
                || normalize_query(&row.city).contains(&normalized_query)
        })
        .count();

    let unique_cities = rows
        .iter()
        .map(|row| row.city.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let unique_types = rows
        .iter()
        .map(|row| row.object_type.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    let lead_city = most_common_value(rows.iter().map(|row| row.city.as_str()));
    let lead_type = most_common_value(rows.iter().map(|row| row.object_type.as_str()));
    let focus_object = rows.iter().min_by_key(|row| {
        let normalized_name = normalize_query(&row.name);
        let exact_penalty = if normalized_name == normalized_query {
            0
        } else {
            1
        };
        let contains_penalty = if normalized_name.contains(&normalized_query) {
            0
        } else {
            1
        };
        (
            exact_penalty,
            contains_penalty,
            normalized_name.len(),
            row.name.as_str(),
        )
    });

    let (confidence_label, confidence_note) = if rows.is_empty() {
        (
            "низкая",
            "По текущему демо-запросу нет совпадений, поэтому аналитика ограничена только входной строкой.",
        )
    } else if exact_match_count > 0 {
        (
            "высокая",
            "Есть точные или почти точные совпадения, поэтому фокус можно строить вокруг найденного объекта без дополнительной фильтрации.",
        )
    } else if rows.len() >= 3 {
        (
            "средняя",
            "Выборка достаточно наполнена для демо-анализа, но запрос пока шире одного объекта и может потребовать уточнения.",
        )
    } else {
        (
            "умеренная",
            "Найдены только частичные совпадения, поэтому вывод стоит использовать как ориентир для следующего шага поиска.",
        )
    };

    QueryAnalysis {
        normalized_query,
        result_count: rows.len(),
        exact_match_count,
        unique_cities,
        unique_types,
        lead_city,
        lead_type,
        focus_object_name: focus_object.map(|row| row.name.clone()),
        focus_object_city: focus_object.map(|row| row.city.clone()),
        confidence_label,
        confidence_note,
    }
}

pub fn validate_pro_package_form(form: &ProPackageForm) -> Vec<String> {
    let mut issues = Vec::new();
    let business_query = collapse_whitespace(&form.business_query);
    let business_category = collapse_whitespace(&form.business_category);

    if business_query.len() < 8 {
        issues.push(
            "Укажите более точный адрес или ориентир, чтобы PRO-анализ понимал геоточку."
                .to_string(),
        );
    }

    if business_category.is_empty() {
        issues.push("Выберите категорию бизнеса перед запуском анализа.".to_string());
    }

    issues
}

pub fn validate_registration_form(form: &RegisterForm) -> Vec<String> {
    let mut errors = Vec::new();

    if collapse_whitespace(&form.full_name).len() < 2 {
        errors.push("Укажите имя пользователя не короче 2 символов.".to_string());
    }

    if normalize_phone(&form.phone).len() < 11 {
        errors
            .push("Укажите корректный телефон в международном или российском формате.".to_string());
    }

    let telegram = normalize_telegram_handle(&form.telegram_handle);
    if telegram.len() < 6 || telegram.len() > 33 {
        errors.push("Укажите корректный ник в Telegram длиной от 5 до 32 символов.".to_string());
    } else if !telegram
        .chars()
        .skip(1)
        .all(|char| char.is_ascii_alphanumeric() || char == '_')
    {
        errors
            .push("Telegram-ник может содержать только латиницу, цифры и символ '_'.".to_string());
    }

    let email = form.email.trim();
    let email_has_shape = email.contains('@')
        && email
            .split_once('@')
            .is_some_and(|(_, domain)| domain.contains('.'));
    if !email_has_shape {
        errors.push("Укажите корректный email.".to_string());
    }

    if form.password.trim().len() < 8 {
        errors.push("Пароль должен содержать не меньше 8 символов.".to_string());
    }

    errors
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn normalize_phone(value: &str) -> String {
    let mut normalized = String::new();
    for char in value.chars() {
        if char.is_ascii_digit() {
            normalized.push(char);
        }
    }

    if normalized.len() == 11 && normalized.starts_with('8') {
        normalized.remove(0);
        normalized.insert(0, '7');
    }

    normalized
}

pub fn normalize_telegram_handle(value: &str) -> String {
    let mut normalized = collapse_whitespace(value).replace(' ', "");
    if !normalized.starts_with('@') {
        normalized.insert(0, '@');
    }
    normalized.to_lowercase()
}

pub fn validation_issue(
    title: impl Into<String>,
    detail: impl Into<String>,
    upgrade_to_pro: bool,
) -> ValidationIssue {
    ValidationIssue {
        title: title.into(),
        detail: detail.into(),
        upgrade_to_pro,
    }
}

pub fn validate_subscription_access(user: &RegisteredUser) -> Option<ValidationIssue> {
    if user.account_tier == "pro" && !user.subscription_is_active {
        return Some(validation_issue(
            "Подписка PRO истекла",
            "Чтобы продолжить анализы, откройте модуль 13 в кабинете и продлите пакет на новый период.",
            true,
        ));
    }

    if user.analysis_quota_used >= user.analysis_quota_limit {
        let detail = if user.account_tier == "pro" {
            "Вы использовали лимит текущего пакета. Чтобы продолжить пользоваться тарифом PRO, необходимо оплатить новый лимит."
        } else {
            "Бесплатный лимит закончился. Чтобы продолжить анализы, откройте модуль 13 в кабинете и подключите PRO-пакет."
        };
        return Some(validation_issue("Лимит запросов исчерпан", detail, true));
    }

    None
}

pub fn build_session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}"
    )
}

pub fn expire_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub fn redirect_with_cookie(location: &str, cookie: &str) -> Response {
    let mut response = Redirect::to(location).into_response();
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

pub fn read_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let trimmed = part.trim();
        if let Some((cookie_name, cookie_value)) = trimmed.split_once('=')
            && cookie_name.trim() == name
        {
            return Some(cookie_value.trim().to_string());
        }
    }
    None
}

pub fn parse_business_profile_url(value: &str) -> Result<ParsedBusinessUrl, String> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed)
        .map_err(|_| "Укажите полную ссылку c `https://` на карточку бизнеса.".to_string())?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Ссылка должна начинаться с `http://` или `https://`.".to_string());
        }
    }

    let host = parsed
        .host_str()
        .map(str::to_lowercase)
        .ok_or_else(|| "Не удалось определить домен ссылки.".to_string())?;
    let path = parsed.path().trim_end_matches('/').to_lowercase();

    let source_platform = if is_yandex_maps_host(&host) {
        if !path.contains("/maps") && !path.contains("/org/") {
            return Err(
                "Для Яндекс Карт нужна ссылка вида `yandex.ru/maps/...` или карточка `.../org/...`."
                    .to_string(),
            );
        }
        MapPlatform::YandexMaps
    } else if is_google_maps_host(&host) {
        if host == "maps.app.goo.gl" {
            if !is_non_root_path(&path) {
                return Err(
                    "Для Google нужна короткая или полная ссылка на карточку из Google Maps."
                        .to_string(),
                );
            }
        } else if !path.contains("/maps") {
            return Err("Для Google нужна ссылка на карточку из Google Maps.".to_string());
        }
        MapPlatform::GoogleMaps
    } else if is_twogis_host(&host) {
        if !is_non_root_path(&path) {
            return Err("Для 2GIS нужна ссылка на конкретную карточку бизнеса.".to_string());
        }
        MapPlatform::TwoGis
    } else {
        return Err(
            "Поддерживаются только реальные ссылки Яндекс Карт, Google Maps и 2GIS.".to_string(),
        );
    };

    let mut canonical = parsed;
    canonical.set_fragment(None);

    Ok(ParsedBusinessUrl {
        source_platform,
        source_url: trimmed.to_string(),
        canonical_url: canonical.to_string(),
    })
}

pub async fn resolve_business_profile_url(
    parsed_url: ParsedBusinessUrl,
) -> Result<ParsedBusinessUrl, ValidationIssue> {
    let snapshot = match fetch_remote_page_snapshot(&parsed_url.source_url, "validation").await {
        Ok(snapshot) => snapshot,
        Err(error) if error.contains("timeout") => {
            return Err(validation_issue(
                "Источник временно недоступен",
                "Сервис карт не ответил вовремя. Попробуйте повторить запрос позже.",
                false,
            ));
        }
        Err(error) if error.contains("connect") => {
            return Err(validation_issue(
                "Попробуйте позже",
                "Сайт не загрузился или не удалось установить соединение с источником.",
                false,
            ));
        }
        Err(_) => {
            return Err(validation_issue(
                "Попробуйте позже",
                "Не удалось проверить карточку по внешнему источнику. Попробуйте ещё раз немного позже.",
                false,
            ));
        }
    };

    let body = snapshot.body.to_lowercase();

    if body.contains("закрыто навсегда")
        || body.contains("temporarily closed")
        || body.contains("permanently closed")
        || body.contains("больше не работает")
        || body.contains("удалена")
    {
        return Err(validation_issue(
            "Карточка недоступна",
            "Похоже, карточка закрыта, удалена или больше не обслуживается источником.",
            false,
        ));
    }

    let final_url = Url::parse(&snapshot.final_url).map_err(|_| {
        validation_issue(
            "Попробуйте позже",
            "Источник вернул неожиданный адрес после редиректа. Повторите попытку немного позже.",
            false,
        )
    })?;

    if !is_business_profile_destination(parsed_url.source_platform, &final_url) {
        return Err(validation_issue(
            "Неверная ссылка",
            "Нужна ссылка именно на карточку бизнеса, а не на общую карту, поиск или промежуточный редирект.",
            false,
        ));
    }

    let mut normalized_url = parsed_url;
    normalized_url.canonical_url = canonicalize_business_profile_url(&final_url);
    Ok(normalized_url)
}

pub async fn fetch_remote_page_snapshot(
    url: &str,
    purpose: &str,
) -> Result<RemotePageSnapshot, String> {
    tokio::time::sleep(request_pacing_delay(url, purpose)).await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(15))
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|error| error.to_string())?;

    let response = client
        .get(url)
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        )
        .header(
            reqwest::header::ACCEPT_LANGUAGE,
            "ru-RU,ru;q=0.9,en-US;q=0.7,en;q=0.6",
        )
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .header(reqwest::header::PRAGMA, "no-cache")
        .header(reqwest::header::REFERER, "https://yandex.ru/maps/")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
        return Err(format!("{purpose}: not found"));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::FORBIDDEN
        || status.is_server_error()
    {
        return Err(format!("{purpose}: source unavailable"));
    }

    let final_url = response.url().to_string();
    let body = response.text().await.map_err(|error| error.to_string())?;
    Ok(RemotePageSnapshot { final_url, body })
}

pub fn browser_worker_enabled() -> bool {
    matches!(
        std::env::var("GEONETIX_BROWSER_WORKER_ENABLED")
            .ok()
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

pub fn live_card_cache_ttl_seconds() -> u64 {
    std::env::var("GEONETIX_LIVE_CARD_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(900)
}

pub fn request_pacing_delay(url: &str, purpose: &str) -> Duration {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    purpose.hash(&mut hasher);
    let jitter_ms = hasher.finish() % 600;
    Duration::from_millis(350 + jitter_ms)
}

pub fn monitoring_event_seed(
    business_query: &str,
    business_category: &str,
) -> [(&'static str, &'static str, String, String); 4] {
    let query_hint = report_excerpt(Some(business_query));
    let category_hint = report_excerpt(Some(business_category));

    [
        (
            "Конкурент X",
            "Изменение рейтинга",
            format!(
                "Рейтинг по сценарию `{category_hint}` вырос с 4.3 до 4.6 рядом с точкой `{query_hint}`."
            ),
            "Проверить последние отзывы и усилить ответы на позитивные кейсы.".to_string(),
        ),
        (
            "Конкурент X",
            "Новые отзывы",
            "За сутки появилось +12 новых отзывов, часть из них усиливает видимость карточки в выдаче.".to_string(),
            "Собрать тональность новых отзывов и подготовить обновление для собственного профиля.".to_string(),
        ),
        (
            "Конкурент Y",
            "Фото и новости",
            "Добавлено 5 новых фото, удалено 0, а также опубликована 1 новость в карточке компании.".to_string(),
            "Подготовить визуальное обновление карточки и короткий инфоповод для публикации.".to_string(),
        ),
        (
            "Конкурент Z",
            "Смена позиций",
            "Карточка сместилась вверх в локальной выдаче и обошла соседние бизнесы по нескольким коммерческим запросам.".to_string(),
            "Проверить, какие поля карточки были обновлены, и повторить сильные паттерны у себя.".to_string(),
        ),
    ]
}

pub fn pdf_escape_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

pub fn sanitize_file_stem(value: &str) -> String {
    let cleaned = transliterate_to_ascii(value)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    cleaned
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn transliterate_to_ascii(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        let replacement = match ch {
            'А' => "A",
            'Б' => "B",
            'В' => "V",
            'Г' => "G",
            'Д' => "D",
            'Е' | 'Ё' => "E",
            'Ж' => "Zh",
            'З' => "Z",
            'И' => "I",
            'Й' => "Y",
            'К' => "K",
            'Л' => "L",
            'М' => "M",
            'Н' => "N",
            'О' => "O",
            'П' => "P",
            'Р' => "R",
            'С' => "S",
            'Т' => "T",
            'У' => "U",
            'Ф' => "F",
            'Х' => "Kh",
            'Ц' => "Ts",
            'Ч' => "Ch",
            'Ш' => "Sh",
            'Щ' => "Sch",
            'Ъ' | 'Ь' => "",
            'Ы' => "Y",
            'Э' => "E",
            'Ю' => "Yu",
            'Я' => "Ya",
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'е' | 'ё' => "e",
            'ж' => "zh",
            'з' => "z",
            'и' => "i",
            'й' => "y",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "sch",
            'ы' => "y",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            _ if ch.is_ascii() => {
                output.push(ch);
                continue;
            }
            _ => "?",
        };
        output.push_str(replacement);
    }
    output
}

pub fn stable_text_score(value: &str) -> usize {
    value.bytes().fold(0usize, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as usize)
    })
}

fn canonicalize_business_profile_url(url: &Url) -> String {
    let mut canonical = url.clone();
    canonical.set_fragment(None);

    // Yandex Maps renders a different (search-result) embedded JSON shape when
    // search/tracking query params (mode=search&text=...&sctx=...) are present
    // on an org card URL, which breaks `parse_yandex_maps_card_from_embedded_state`.
    // Share/short links redirect to the org path with this query string attached,
    // so it must be stripped to keep the plain "results.items" state shape.
    let host = canonical
        .host_str()
        .map(str::to_lowercase)
        .unwrap_or_default();
    if is_yandex_maps_host(&host) && canonical.path().contains("/org/") {
        canonical.set_query(None);
    }

    canonical.to_string()
}

fn is_non_root_path(path: &str) -> bool {
    !path.is_empty() && path != "/"
}

fn is_google_maps_host(host: &str) -> bool {
    host == "maps.app.goo.gl"
        || host == "google.com"
        || host.ends_with(".google.com")
        || host.starts_with("maps.google.")
}

fn is_twogis_host(host: &str) -> bool {
    host == "2gis.ru"
        || host.ends_with(".2gis.ru")
        || host == "go.2gis.com"
        || host == "dgis.ru"
        || host.ends_with(".dgis.ru")
}

fn is_yandex_maps_host(host: &str) -> bool {
    host == "yandex.ru"
        || host.ends_with(".yandex.ru")
        || host == "yandex.com"
        || host.ends_with(".yandex.com")
}

fn is_business_profile_destination(platform: MapPlatform, url: &Url) -> bool {
    let Some(host) = url.host_str().map(str::to_lowercase) else {
        return false;
    };
    let path = url.path().trim_end_matches('/').to_lowercase();

    match platform {
        MapPlatform::YandexMaps => {
            if !is_yandex_maps_host(&host) {
                return false;
            }

            if path.contains("/org/") || path.contains("/maps/-/") {
                return true;
            }

            let mut has_mode_poi = false;
            let mut has_business_poi = false;

            for (key, value) in url.query_pairs() {
                if key == "mode" && value == "poi" {
                    has_mode_poi = true;
                }

                if (key == "oid" && !value.is_empty())
                    || (key == "poi[uri]" && value.starts_with("ymapsbm1:/org"))
                    || (key == "poi[point]" && !value.is_empty())
                {
                    has_business_poi = true;
                }
            }

            has_mode_poi && has_business_poi
        }
        MapPlatform::GoogleMaps => {
            if !is_google_maps_host(&host) {
                return false;
            }

            if host == "maps.app.goo.gl" {
                return is_non_root_path(&path);
            }

            path.contains("/maps/place")
                || url.query_pairs().any(|(key, value)| {
                    matches!(key.as_ref(), "cid" | "query_place_id") && !value.is_empty()
                })
        }
        MapPlatform::TwoGis => {
            if !is_twogis_host(&host) {
                return false;
            }

            is_non_root_path(&path)
        }
    }
}

pub fn platform_label_from_db(value: &str) -> &'static str {
    match value {
        "yandex_maps" => "Яндекс Карты",
        "google_maps" => "Google Maps",
        "two_gis" => "2GIS",
        _ => "Неизвестная платформа",
    }
}

pub fn monitoring_status_label(value: &str) -> &'static str {
    match value {
        "active" => "активен",
        "paused" => "на паузе",
        _ => "неизвестно",
    }
}

pub fn monitoring_frequency_label(value: &str) -> &'static str {
    match value {
        "daily" => "ежедневно",
        _ => "по расписанию",
    }
}

pub fn report_format_label(value: &str) -> &'static str {
    match value {
        "pdf" => "PDF",
        _ => "по умолчанию",
    }
}

pub fn request_status_label(value: &str) -> &'static str {
    match value {
        "queued" => "в очереди",
        "processing" => "выполняется",
        "completed" => "готов",
        "failed" => "ошибка",
        _ => "неизвестно",
    }
}

pub fn subscription_plan_label(value: &str) -> &'static str {
    match value {
        "free" => "Бесплатный",
        "pro_3" => "PRO 3",
        "pro_10" => "PRO 10",
        _ => "Неизвестный тариф",
    }
}

pub fn subscription_status_label(user: &RegisteredUser) -> &'static str {
    if user.account_tier == "pro" && user.subscription_is_active {
        "Активна"
    } else if user.account_tier == "pro" {
        "Истекла"
    } else {
        "Не подключена"
    }
}

pub fn subscription_end_label(user: &RegisteredUser) -> String {
    match user.subscription_expires_at_label.as_deref() {
        Some(label) if !label.is_empty() => format!("до {label}"),
        _ => "не указан".to_string(),
    }
}

pub fn module_eight_stage_label(value: &str) -> &'static str {
    match value {
        "validate_request" => "Проверяем запрос",
        "open_chrome" => "Открываем карточку",
        "inspect_card" => "Разбираем карточку",
        "analyze_reviews" => "Анализируем отзывы",
        "compare_competitors" => "Сравниваем конкурентов",
        "publish_report" => "Публикуем отчёт",
        _ => "Обработка...",
    }
}

pub fn stage_label_from_key(value: &str) -> &'static str {
    match value {
        "validate_link" => "Проверяем ссылку",
        "load_data" => "Получаем данные",
        "analyze_reviews" => "Анализируем отзывы",
        "compare_competitors" => "Сравниваем конкурентов",
        "build_report" => "Формируем отчёт",
        "save_result" => "Сохраняем результат",
        _ => "Обработка...",
    }
}
