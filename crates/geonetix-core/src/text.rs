pub fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn report_excerpt(value: Option<&str>) -> String {
    let text = value
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Подробность появится после расширенного PRO-анализа.".to_string());

    if text.chars().count() <= 140 {
        text
    } else {
        let shortened: String = text.chars().take(137).collect();
        format!("{shortened}...")
    }
}
