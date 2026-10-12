use geonetix_parse_jsonld::{extract_json_ld_number, extract_json_ld_text};
use scraper::Html as ScraperHtml;

fn doc(body: &str) -> ScraperHtml {
    ScraperHtml::parse_document(body)
}

#[test]
fn reads_a_flat_name() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@type":"CafeOrCoffeeShop","name":"Кофейня Сварка"}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Кофейня Сварка".to_string())
    );
}

#[test]
fn reads_a_nested_address() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"Restaurant","address":{"@type":"PostalAddress","streetAddress":"ул. Тверская, 12"}}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["address", "streetAddress"]),
        Some("ул. Тверская, 12".to_string())
    );
}

#[test]
fn reads_rating_and_review_count() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"CafeOrCoffeeShop","aggregateRating":{"ratingValue":4.8,"reviewCount":217}}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "ratingValue"]),
        Some(4.8)
    );
    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "reviewCount"]),
        Some(217.0)
    );
}

#[test]
fn accepts_rating_count_as_a_review_count_alias() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"CafeOrCoffeeShop","aggregateRating":{"ratingValue":4.5,"ratingCount":96}}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "ratingCount"]),
        Some(96.0)
    );
}

#[test]
fn parses_a_rating_written_with_a_comma() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"CafeOrCoffeeShop","aggregateRating":{"ratingValue":"4,7"}}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "ratingValue"]),
        Some(4.7)
    );
}

#[test]
fn walks_into_graph_arrays_with_an_explicit_path() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[
          {"@type":"WebSite","name":"Сайт"},
          {"@type":"Restaurant","name":"Ресторан Второй"}
        ]}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["@graph", "name"]),
        Some("Сайт".to_string())
    );
    assert_eq!(
        extract_json_ld_text(&document, &["@graph", "@type"]),
        Some("WebSite".to_string())
    );
}

#[test]
fn reaches_entities_wrapped_in_a_graph_without_an_explicit_path() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[
          {"@type":"Restaurant","name":"Ресторан в @graph"}
        ]}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Ресторан в @graph".to_string())
    );
    assert_eq!(
        extract_json_ld_text(&document, &["@type"]),
        Some("Restaurant".to_string())
    );
}

#[test]
fn prefers_a_top_level_key_over_a_graph_entity() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","name":"Верхний уровень","@graph":[
          {"@type":"Restaurant","name":"Внутри графа"}
        ]}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Верхний уровень".to_string())
    );
}

#[test]
fn walks_a_business_wrapped_in_a_graph_like_google_requires() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[
          {"@type":"WebPage","name":"Страница"},
          {"@type":"Restaurant","name":"Кофейня Сварка",
           "address":{"@type":"PostalAddress","streetAddress":"ул. Тверская, 12"},
           "aggregateRating":{"ratingValue":4.8,"reviewCount":217}}
        ]}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Страница".to_string())
    );
    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "ratingValue"]),
        Some(4.8)
    );
    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "reviewCount"]),
        Some(217.0)
    );
}

#[test]
fn walks_into_top_level_arrays() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        [{"@type":"Organization","name":"Первый"},
         {"@type":"Organization","name":"Второй"}]
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Первый".to_string())
    );
}

#[test]
fn skips_a_malformed_script_and_keeps_reading() {
    let document = doc(r#"<html><head>
        <script type="application/ld+json">{ this is not json }</script>
        <script type="application/ld+json">{"@type":"CafeOrCoffeeShop","name":"Рабочая карточка"}</script>
        </head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Рабочая карточка".to_string())
    );
}

#[test]
fn ignores_scripts_that_are_not_json_ld() {
    let document = doc(r#"<html><head>
        <script type="application/javascript">var name = "Не должна читаться";</script>
        </head><body></body></html>"#);

    assert_eq!(extract_json_ld_text(&document, &["name"]), None);
}

#[test]
fn collapses_whitespace_in_extracted_text() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"CafeOrCoffeeShop","name":"  Кофе   и\n  десерты "}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Кофе и десерты".to_string())
    );
}

#[test]
fn skips_a_script_whose_json_string_contains_a_raw_newline() {
    let document = doc("<html><head><script type=\"application/ld+json\">\
         {\"@type\":\"CafeOrCoffeeShop\",\"name\":\"Кофе\nи десерты\"}\
         </script></head><body></body></html>");

    assert_eq!(extract_json_ld_text(&document, &["name"]), None);
}

#[test]
fn returns_none_for_a_missing_path() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"CafeOrCoffeeShop","name":"Без адреса"}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["address", "streetAddress"]),
        None
    );
    assert_eq!(
        extract_json_ld_number(&document, &["aggregateRating", "ratingValue"]),
        None
    );
}

#[test]
fn returns_none_when_the_document_has_no_json_ld() {
    let document = doc("<html><head><title>Пусто</title></head><body>текст</body></html>");

    assert_eq!(extract_json_ld_text(&document, &["name"]), None);
    assert_eq!(extract_json_ld_number(&document, &["rating"]), None);
}

#[test]
fn prefers_the_first_script_that_satisfies_the_path() {
    let document = doc(r#"<html><head>
        <script type="application/ld+json">{"@type":"CafeOrCoffeeShop","name":"Первый"}</script>
        <script type="application/ld+json">{"@type":"CafeOrCoffeeShop","name":"Второй"}</script>
        </head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["name"]),
        Some("Первый".to_string())
    );
}

#[test]
fn reads_a_url_field() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"Restaurant","name":"Ресторан","url":"https://example.com"}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["url"]),
        Some("https://example.com".to_string())
    );
}

#[test]
fn reads_a_description_field() {
    let document = doc(r#"<html><head><script type="application/ld+json">
        {"@type":"Restaurant","name":"Ресторан","description":"Уютное место"}
        </script></head><body></body></html>"#);

    assert_eq!(
        extract_json_ld_text(&document, &["description"]),
        Some("Уютное место".to_string())
    );
}
