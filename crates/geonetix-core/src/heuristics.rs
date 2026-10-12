use crate::text::collapse_whitespace;

pub fn count_keyword_hits(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .count()
}

pub fn estimate_sentiment_shares(text: &str) -> (i32, i32) {
    let positive_hits = count_keyword_hits(
        text,
        &[
            "отлич",
            "супер",
            "вкусн",
            "приятн",
            "рекоменд",
            "good",
            "great",
            "excellent",
            "amazing",
        ],
    ) as i32;
    let negative_hits = count_keyword_hits(
        text,
        &[
            "плохо",
            "ужас",
            "долго",
            "грязн",
            "хам",
            "bad",
            "poor",
            "terrible",
            "awful",
            "worst",
        ],
    ) as i32;

    let total = positive_hits + negative_hits;
    if total == 0 {
        return (50, 10);
    }

    let positive = ((positive_hits * 100) / total).clamp(5, 95);
    let negative = ((negative_hits * 100) / total).clamp(0, 80);
    (positive, negative)
}

pub fn first_non_empty(values: Vec<Option<String>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(|value| collapse_whitespace(&value))
        .find(|value| !value.is_empty())
}
