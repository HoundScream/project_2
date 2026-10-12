use crate::{MapPlatform, text::collapse_whitespace};
use url::Url;

pub fn platform_seed_name(platform: MapPlatform) -> String {
    match platform {
        MapPlatform::YandexMaps => "Профиль".to_string(),
        MapPlatform::GoogleMaps => "Карточка".to_string(),
        MapPlatform::TwoGis => "Бизнес-профиль".to_string(),
    }
}

pub fn usable_card_title(platform: MapPlatform, value: &str) -> Option<String> {
    let normalized = collapse_whitespace(value);
    if normalized.is_empty() {
        return None;
    }

    let lowercase = normalized.to_lowercase();
    let is_shell_title = match platform {
        MapPlatform::GoogleMaps => matches!(
            lowercase.as_str(),
            "google maps" | "google карты" | "карты google"
        ),
        MapPlatform::YandexMaps => matches!(lowercase.as_str(), "яндекс карты" | "yandex maps"),
        MapPlatform::TwoGis => matches!(lowercase.as_str(), "2гис" | "2gis"),
    };

    if is_shell_title {
        None
    } else {
        Some(normalized)
    }
}

pub fn google_place_name_from_url(url: &Url) -> Option<String> {
    let segments = url.path_segments()?.collect::<Vec<_>>();
    let place_index = segments.iter().position(|segment| *segment == "place")?;
    let raw_name = segments.get(place_index + 1)?.trim();
    if raw_name.is_empty() {
        return None;
    }
    let decoded = raw_name.replace('+', " ");
    let normalized = collapse_whitespace(&decoded);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.push_str(&chars.as_str().to_lowercase());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn infer_profile_title(platform: MapPlatform, canonical_url: &str) -> String {
    let parsed = match Url::parse(canonical_url) {
        Ok(url) => url,
        Err(_) => return platform_seed_name(platform),
    };

    if matches!(platform, MapPlatform::GoogleMaps)
        && let Some(place_name) = google_place_name_from_url(&parsed)
    {
        return place_name;
    }

    let slug = parsed
        .path_segments()
        .into_iter()
        .flatten()
        .rfind(|segment| !segment.is_empty())
        .unwrap_or_default();
    let normalized = slug
        .replace(['-', '_'], " ")
        .replace("%20", " ")
        .trim()
        .to_string();

    if normalized.is_empty() {
        platform_seed_name(platform)
    } else {
        format!(
            "{} {}",
            platform_seed_name(platform),
            title_case(&normalized)
        )
    }
}

pub fn infer_profile_address(platform: MapPlatform, canonical_url: &str) -> String {
    let zone = match platform {
        MapPlatform::YandexMaps => "локальный кластер Яндекс Карт",
        MapPlatform::GoogleMaps => "кластер Google Maps",
        MapPlatform::TwoGis => "кластер 2GIS",
    };
    let parsed = Url::parse(canonical_url).ok();
    let host = parsed
        .as_ref()
        .and_then(Url::host_str)
        .unwrap_or("unknown-host");
    format!("Источник: {host}, сегмент: {zone}")
}
