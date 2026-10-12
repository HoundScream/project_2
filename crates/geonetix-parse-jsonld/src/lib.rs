use geonetix_core::text::collapse_whitespace;
use scraper::{Html as ScraperHtml, Selector};
use serde_json::Value;

pub fn extract_json_ld_text(document: &ScraperHtml, path: &[&str]) -> Option<String> {
    for value in extract_json_ld_values(document) {
        if let Some(text) = find_json_path_text(&value, path) {
            let normalized = collapse_whitespace(&text);
            if !normalized.is_empty() {
                return Some(normalized);
            }
        }
    }
    None
}

pub fn extract_json_ld_number(document: &ScraperHtml, path: &[&str]) -> Option<f32> {
    for value in extract_json_ld_values(document) {
        if let Some(number) = find_json_path_number(&value, path) {
            return Some(number);
        }
    }
    None
}

pub fn extract_json_ld_values(document: &ScraperHtml) -> Vec<Value> {
    let Ok(selector) = Selector::parse(r#"script[type="application/ld+json"]"#) else {
        return Vec::new();
    };

    document
        .select(&selector)
        .filter_map(|node| serde_json::from_str::<Value>(&node.inner_html()).ok())
        .collect()
}

pub fn find_json_path_text(value: &Value, path: &[&str]) -> Option<String> {
    let node = find_json_path_node(value, path)?;
    match node {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(values) => values
            .iter()
            .find_map(|item| find_json_path_text(item, &[])),
        Value::Object(_) if path.is_empty() => node
            .get("name")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        _ => None,
    }
}

pub fn find_json_path_number(value: &Value, path: &[&str]) -> Option<f32> {
    let node = find_json_path_node(value, path)?;
    match node {
        Value::Number(number) => number.as_f64().map(|value| value as f32),
        Value::String(text) => text.replace(',', ".").parse::<f32>().ok(),
        _ => None,
    }
}

pub fn find_json_path_node<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }

    match value {
        Value::Object(map) => map
            .get(path[0])
            .and_then(|next| find_json_path_node(next, &path[1..]))
            .or_else(|| {
                map.get("@graph")
                    .and_then(|graph| find_json_path_node(graph, path))
            }),
        Value::Array(values) => values
            .iter()
            .find_map(|item| find_json_path_node(item, path)),
        _ => None,
    }
}
