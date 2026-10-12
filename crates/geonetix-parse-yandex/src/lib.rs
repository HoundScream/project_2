use geonetix_core::heuristics::{estimate_sentiment_shares, first_non_empty};
use geonetix_core::platform::{
    infer_profile_address, infer_profile_title, platform_seed_name, usable_card_title,
};
use geonetix_core::text::{collapse_whitespace, report_excerpt};
use geonetix_core::{LiveMapCardData, MapPlatform, RemotePageSnapshot};
use geonetix_parse_jsonld::{find_json_path_node, find_json_path_number, find_json_path_text};
use scraper::{Html as ScraperHtml, Selector};
use serde_json::Value;

pub fn parse_yandex_maps_card_from_embedded_state(
    canonical_url: &str,
    snapshot: &RemotePageSnapshot,
) -> Option<LiveMapCardData> {
    let document = ScraperHtml::parse_document(&snapshot.body);
    let body_text = collapse_whitespace(
        &Selector::parse("body")
            .ok()
            .and_then(|selector| document.select(&selector).next())
            .map(|node| node.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default(),
    );
    let body_text_lower = body_text.to_lowercase();
    let state = extract_yandex_maps_state(&document)?;
    let item = extract_yandex_maps_business_item(&state)?;

    let category_hint = first_non_empty(vec![
        find_json_path_text(item, &["categories", "name"]),
        find_json_path_text(item, &["topObjects", "rubric", "name"]),
        Some(platform_seed_name(MapPlatform::YandexMaps)),
    ])
    .unwrap_or_else(|| platform_seed_name(MapPlatform::YandexMaps));
    let working_time = find_json_path_text(item, &["workingTimeText"]);
    let current_status = find_json_path_text(item, &["currentWorkingStatus", "text"]);
    let additional_address = find_json_path_text(item, &["additionalAddress"]);

    let title = first_non_empty(vec![
        find_json_path_text(item, &["title"])
            .and_then(|value| usable_card_title(MapPlatform::YandexMaps, &value)),
        find_json_path_text(item, &["shortTitle"])
            .and_then(|value| usable_card_title(MapPlatform::YandexMaps, &value)),
        Some(infer_profile_title(MapPlatform::YandexMaps, canonical_url)),
    ])
    .unwrap_or_else(|| infer_profile_title(MapPlatform::YandexMaps, canonical_url));

    let address = first_non_empty(vec![
        find_json_path_text(item, &["fullAddress"]),
        find_json_path_text(item, &["address"]),
        find_json_path_text(item, &["description"]),
        Some(infer_profile_address(
            MapPlatform::YandexMaps,
            canonical_url,
        )),
    ])
    .unwrap_or_else(|| infer_profile_address(MapPlatform::YandexMaps, canonical_url));

    let description = first_non_empty(vec![
        join_non_empty_parts(&[
            Some(category_hint.clone()),
            current_status.clone(),
            working_time.clone(),
            additional_address.clone(),
        ]),
        Some(address.clone()),
    ])
    .unwrap_or_else(|| address.clone());

    let rating = find_json_path_number(item, &["ratingData", "ratingValue"]);
    let review_count = find_json_path_number(item, &["ratingData", "reviewCount"])
        .or_else(|| find_json_path_number(item, &["ratingData", "ratingCount"]))
        .map(|value| value.round() as i32);
    let photo_count = yandex_photo_count(item).clamp(0, 200) as i32;
    let menu_items_count = yandex_menu_items_count(item).clamp(0, 50) as i32;
    let website_url = yandex_external_website_url(item);
    let contact_count = (yandex_array_len(item, &["phones"])
        + yandex_array_len(item, &["businessLinks"])
        + website_url.as_ref().map(|_| 1).unwrap_or(0))
    .clamp(0, 12) as i32;
    let social_links = yandex_social_links(item);
    let social_count = yandex_array_len(item, &["socialLinks"]).clamp(0, 10) as i32;
    let feature_count = yandex_enabled_feature_count(item).clamp(0, 20) as i32;
    let has_delivery = yandex_has_delivery(item);
    let (positive_review_share, negative_review_share) = yandex_review_sentiment(item)
        .unwrap_or_else(|| estimate_sentiment_shares(&body_text_lower));

    let mut diagnostics = vec![
        format!("Источник ответа: {}", snapshot.final_url),
        "Карточка Яндекс Карт разобрана через embedded JSON state страницы.".to_string(),
        format!(
            "Категория карточки: {}",
            report_excerpt(Some(&category_hint))
        ),
    ];
    if let Some(rating) = rating {
        diagnostics.push(format!("Обнаружен рейтинг карточки: {rating:.1}/5"));
    }
    if let Some(review_count) = review_count {
        diagnostics.push(format!("Число отзывов в Yandex state: {review_count}"));
    }
    if let Some(working_time) = working_time {
        diagnostics.push(format!(
            "График работы: {}",
            report_excerpt(Some(&working_time))
        ));
    }
    if website_url.is_none() {
        diagnostics.push("Во встроенном state не найден внешний сайт бизнеса.".to_string());
    }

    Some(LiveMapCardData {
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
    })
}

fn extract_yandex_maps_state(document: &ScraperHtml) -> Option<Value> {
    let selector = Selector::parse("script").ok()?;

    document.select(&selector).find_map(|node| {
        let content = node.inner_html();
        let trimmed = content.trim();
        if !trimmed.starts_with('{') {
            return None;
        }

        let parsed = serde_json::from_str::<Value>(trimmed).ok()?;
        let has_stack = matches!(parsed.get("stack"), Some(Value::Array(_)));
        let has_results = find_json_path_node(&parsed, &["stack", "results", "items"]).is_some();
        if has_stack && has_results {
            Some(parsed)
        } else {
            None
        }
    })
}

fn extract_yandex_maps_business_item(state: &Value) -> Option<&Value> {
    find_json_path_node(state, &["stack", "results", "items"]).and_then(|items| match items {
        Value::Array(values) => values.iter().find(|value| {
            value
                .get("type")
                .and_then(Value::as_str)
                .map(|kind| kind == "business")
                .unwrap_or(false)
        }),
        _ => None,
    })
}

fn yandex_array_len(value: &Value, path: &[&str]) -> usize {
    match find_json_path_node(value, path) {
        Some(Value::Array(values)) => values.len(),
        _ => 0,
    }
}

fn yandex_photo_count(item: &Value) -> usize {
    find_json_path_number(item, &["photos", "count"])
        .map(|value| value.round() as usize)
        .or_else(|| match find_json_path_node(item, &["photos", "items"]) {
            Some(Value::Array(values)) => Some(values.len()),
            _ => None,
        })
        .unwrap_or(0)
}

fn yandex_menu_items_count(item: &Value) -> usize {
    let category_items = match find_json_path_node(item, &["topObjects", "categories"]) {
        Some(Value::Array(categories)) => categories
            .iter()
            .filter_map(|category| match category.get("categoryItems") {
                Some(Value::Array(items)) => Some(items.len()),
                _ => None,
            })
            .sum(),
        _ => 0,
    };

    let menu_features = match find_json_path_node(item, &["features"]) {
        Some(Value::Array(features)) => features
            .iter()
            .filter(|feature| {
                let feature_id = feature
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_lowercase();
                let feature_name = feature
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_lowercase();

                feature_id.contains("menu")
                    || feature_id.contains("catalog")
                    || feature_id.contains("price")
                    || feature_name.contains("меню")
                    || feature_name.contains("каталог")
                    || feature_name.contains("прайс")
                    || feature_name.contains("цена")
            })
            .count(),
        _ => 0,
    };

    category_items.max(menu_features)
}

fn yandex_enabled_feature_count(item: &Value) -> usize {
    match find_json_path_node(item, &["features"]) {
        Some(Value::Array(features)) => features
            .iter()
            .filter(|feature| match feature.get("value") {
                Some(Value::Bool(enabled)) => *enabled,
                Some(Value::String(text)) => !collapse_whitespace(text).is_empty(),
                Some(Value::Array(values)) => !values.is_empty(),
                Some(Value::Number(_)) => true,
                _ => false,
            })
            .count(),
        _ => 0,
    }
}

fn yandex_social_links(item: &Value) -> Vec<String> {
    match find_json_path_node(item, &["socialLinks"]) {
        Some(Value::Array(links)) => links
            .iter()
            .filter_map(|link| {
                link.get("url")
                    .or_else(|| link.get("href"))
                    .or_else(|| link.get("link"))
                    .and_then(Value::as_str)
                    .map(collapse_whitespace)
                    .filter(|url| !url.is_empty())
            })
            .take(10)
            .collect(),
        _ => Vec::new(),
    }
}

fn yandex_has_delivery(item: &Value) -> bool {
    match find_json_path_node(item, &["features"]) {
        Some(Value::Array(features)) => features.iter().any(|feature| {
            let feature_id = feature
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            let feature_name = feature
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            feature_id.contains("delivery")
                || feature_name.contains("доставка")
                || feature_name.contains("delivery")
        }),
        _ => false,
    }
}

fn yandex_review_sentiment(item: &Value) -> Option<(i32, i32)> {
    if let Some(Value::Array(aspects)) = find_json_path_node(item, &["aspects"]) {
        let (positive, neutral, negative) =
            aspects.iter().fold((0i32, 0i32, 0i32), |acc, aspect| {
                (
                    acc.0
                        + aspect
                            .get("positive")
                            .and_then(Value::as_i64)
                            .unwrap_or_default() as i32,
                    acc.1
                        + aspect
                            .get("neutral")
                            .and_then(Value::as_i64)
                            .unwrap_or_default() as i32,
                    acc.2
                        + aspect
                            .get("negative")
                            .and_then(Value::as_i64)
                            .unwrap_or_default() as i32,
                )
            });
        let total = positive + neutral + negative;
        if total > 0 {
            return Some((
                ((positive * 100) / total).clamp(5, 95),
                ((negative * 100) / total).clamp(0, 80),
            ));
        }
    }

    if let Some(Value::Array(reviews)) = find_json_path_node(item, &["reviewResults", "reviews"]) {
        let mut positive = 0i32;
        let mut negative = 0i32;
        for review in reviews {
            let rating = review
                .get("rating")
                .and_then(Value::as_i64)
                .unwrap_or_default() as i32;
            if rating >= 4 {
                positive += 1;
            } else if rating > 0 && rating <= 2 {
                negative += 1;
            }
        }
        let total = positive + negative;
        if total > 0 {
            return Some((
                ((positive * 100) / total).clamp(5, 95),
                ((negative * 100) / total).clamp(0, 80),
            ));
        }
    }

    None
}

fn yandex_external_website_url(item: &Value) -> Option<String> {
    let candidates = [
        find_json_path_text(item, &["urls"]),
        find_json_path_text(item, &["businessLinks", "href"]),
        find_json_path_text(item, &["sources", "href"]),
        find_json_path_text(item, &["source", "href"]),
        find_json_path_text(item, &["url"]),
    ];

    candidates.into_iter().flatten().find(|url| {
        let lower = url.to_lowercase();
        (lower.starts_with("http://") || lower.starts_with("https://"))
            && !lower.contains("yandex.")
            && !lower.contains("ya.ru")
            && !lower.contains("yastatic.net")
            && !lower.contains("eda.yandex")
            && !lower.contains("2gis.ru")
            && !lower.contains("maps.yandex")
    })
}

fn join_non_empty_parts(parts: &[Option<String>]) -> Option<String> {
    let values = parts
        .iter()
        .flatten()
        .map(|value| collapse_whitespace(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values.join(". "))
    }
}
