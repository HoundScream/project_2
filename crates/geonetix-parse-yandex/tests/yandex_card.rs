use geonetix_core::RemotePageSnapshot;
use geonetix_parse_yandex::parse_yandex_maps_card_from_embedded_state;

const URL: &str = "https://yandex.ru/maps/org/hookahplace_akari/25558422006/";

fn card_from_state(state: &str) -> Option<geonetix_core::LiveMapCardData> {
    card_from_body(&format!(
        r#"<html><head><script type="application/json">{state}</script></head><body>Резервный текст страницы</body></html>"#
    ))
}

fn card_from_body(body: &str) -> Option<geonetix_core::LiveMapCardData> {
    let snapshot = RemotePageSnapshot {
        final_url: URL.to_string(),
        body: body.to_string(),
    };
    parse_yandex_maps_card_from_embedded_state(URL, &snapshot)
}

fn fixture_state() -> String {
    std::fs::read_to_string("tests/fixtures/yandex-card.json").expect("fixture must be readable")
}

fn state_with_item(item: &str) -> String {
    format!(r#"{{"stack":[{{"results":{{"items":[{item}]}}}}]}}"#)
}

const BUSINESS: &str = r#""type":"business""#;
const BARE_ITEM: &str = r#"{"type":"business"}"#;

#[test]
fn reads_the_real_recording() {
    let card = card_from_state(&fixture_state()).expect("real card must parse");

    assert_eq!(card.card_title, "HookahPlace Akari");
    assert_eq!(card.address, "Москва, Большой Овчинниковский переулок, 16");
    assert_eq!(card.category_hint, "Кальян-бар");
    assert_eq!(card.rating, Some(5.0));
    assert_eq!(card.review_count, Some(150));
    assert_eq!(card.photo_count, 183);
    assert_eq!(card.menu_items_count, 8);
    assert_eq!(card.contact_count, 3);
    assert_eq!(card.social_count, 2);
    assert_eq!(card.feature_count, 20);
    assert_eq!(card.positive_review_share, 89);
    assert_eq!(card.negative_review_share, 5);
    assert_eq!(card.website_url.as_deref(), Some("https://hookahplace.ru/"));
    assert!(card.has_delivery);
    assert_eq!(
        card.social_links,
        vec![
            "https://t.me/+9sKpMGy9LhFmMDli".to_string(),
            "https://vk.ru/hookahplace_akari".to_string(),
        ]
    );
}

#[test]
fn builds_the_description_from_status_hours_and_entrance() {
    let card = card_from_state(&fixture_state()).expect("card must parse");

    assert_eq!(
        card.description,
        "Кальян-бар. Открыто до 02:00. ежедневно, 11:00–02:00. \
         вход с фудкорта, и отдельный вход с улицы - с торца, ТЦ, этаж 2"
    );
}

#[test]
fn records_diagnostics_about_the_source() {
    let card = card_from_state(&fixture_state()).expect("card must parse");
    let joined = card.diagnostics.join(" | ");

    assert!(joined.contains("embedded JSON state"));
    assert!(joined.contains(URL));
    assert!(joined.contains("Категория карточки: Кальян-бар"));
    assert!(joined.contains("График работы"));
}

#[test]
fn returns_none_when_the_page_has_no_state() {
    assert!(card_from_body("<html><body>Карточка не найдена</body></html>").is_none());
}

#[test]
fn returns_none_when_the_script_is_not_json() {
    let body = r#"<html><head><script>{ not json at all }</script></head><body></body></html>"#;
    assert!(card_from_body(body).is_none());
}

#[test]
fn returns_none_when_the_state_has_no_items() {
    assert!(card_from_state(r#"{"stack":[{"results":{}}]}"#).is_none());
    assert!(card_from_state(r#"{"stack":[]}"#).is_none());
    assert!(card_from_state(r#"{"config":{}}"#).is_none());
}

#[test]
fn returns_none_when_no_item_is_marked_as_a_business() {
    let state = state_with_item(r#"{"type":"advertisement","title":"Реклама"}"#);
    assert!(card_from_state(&state).is_none());
}

#[test]
fn reads_the_state_out_of_a_top_level_array_of_scripts() {
    let body = format!(
        r#"<html><head><script>{{"config":{{}}}}</script><script>{}</script></head><body></body></html>"#,
        fixture_state()
    );
    assert!(card_from_body(&body).is_some());
}

#[test]
fn prefers_review_count_over_rating_count() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"ratingData":{{"ratingValue":4.5,"reviewCount":10,"ratingCount":99}}}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(card.rating, Some(4.5));
    assert_eq!(card.review_count, Some(10));
}

#[test]
fn falls_back_to_rating_count() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"ratingData":{{"ratingValue":4.5,"ratingCount":99}}}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(card.review_count, Some(99));
}

#[test]
fn counts_photo_items_when_the_count_is_absent() {
    let with_count = state_with_item(&format!(
        r#"{{{BUSINESS},"photos":{{"count":12,"items":[{{}},{{}},{{}}]}}}}"#
    ));
    assert_eq!(card_from_state(&with_count).unwrap().photo_count, 12);

    let without_count = state_with_item(&format!(
        r#"{{{BUSINESS},"photos":{{"items":[{{}},{{}}]}}}}"#
    ));
    assert_eq!(card_from_state(&without_count).unwrap().photo_count, 2);
}

#[test]
fn counts_menu_items_from_categories() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"topObjects":{{"categories":[
            {{"categoryName":"menu","categoryItems":[{{"title":"1"}},{{"title":"2"}},{{"title":"3"}}]}},
            {{"categoryName":"more","categoryItems":[{{"title":"4"}}]}}
        ]}}}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().menu_items_count, 4);
}

#[test]
fn counts_menu_features_when_categories_are_absent() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"features":[
            {{"id":"menu","value":true}},
            {{"id":"price_list","value":true}},
            {{"id":"wi_fi","value":true}}
        ]}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().menu_items_count, 2);
}

#[test]
fn detects_delivery_from_a_feature_name() {
    let by_id = state_with_item(&format!(
        r#"{{{BUSINESS},"features":[{{"id":"delivery","value":true}}]}}"#
    ));
    assert!(card_from_state(&by_id).unwrap().has_delivery);

    let by_name = state_with_item(&format!(
        r#"{{{BUSINESS},"features":[{{"id":"takeaway","name":"Доставка еды","value":true}}]}}"#
    ));
    assert!(card_from_state(&by_name).unwrap().has_delivery);

    let without = state_with_item(&format!(
        r#"{{{BUSINESS},"features":[{{"id":"wi_fi","value":true}}]}}"#
    ));
    assert!(!card_from_state(&without).unwrap().has_delivery);
}

#[test]
fn derives_sentiment_from_aspects() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"aspects":[
            {{"positive":90,"neutral":5,"negative":5}},
            {{"positive":10,"neutral":0,"negative":10}}
        ]}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(
        (card.positive_review_share, card.negative_review_share),
        (83, 12)
    );
}

#[test]
fn derives_sentiment_from_review_ratings_without_aspects() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"reviewResults":{{"reviews":[
            {{"rating":5}},{{"rating":4}},{{"rating":5}},{{"rating":1}}
        ]}}}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(
        (card.positive_review_share, card.negative_review_share),
        (75, 25)
    );
}

#[test]
fn prefers_the_first_url_over_business_links() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"urls":["https://example.com/"],"businessLinks":[{{"href":"https://booking.example"}}]}}"#
    ));
    assert_eq!(
        card_from_state(&state).unwrap().website_url.as_deref(),
        Some("https://example.com/")
    );
}

#[test]
fn falls_back_to_a_business_link_then_to_sources() {
    let link = state_with_item(&format!(
        r#"{{{BUSINESS},"businessLinks":[{{"href":"https://booking.example"}}]}}"#
    ));
    assert_eq!(
        card_from_state(&link).unwrap().website_url.as_deref(),
        Some("https://booking.example")
    );

    let source = state_with_item(&format!(
        r#"{{{BUSINESS},"sources":[{{"name":"Владелец","href":"https://business.example/"}}]}}"#
    ));
    assert_eq!(
        card_from_state(&source).unwrap().website_url.as_deref(),
        Some("https://business.example/")
    );
}

#[test]
fn ignores_the_platforms_own_links_when_looking_for_a_website() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"urls":["https://yandex.ru/maps/"],
            "sources":[{{"name":"Яндекс","href":"https://ya.ru/"}}]}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert!(card.website_url.is_none());
    assert_eq!(card.contact_count, 0);
}

#[test]
fn notes_a_missing_external_website() {
    let state = state_with_item(BARE_ITEM);
    let card = card_from_state(&state).expect("card must parse");

    assert!(card.website_url.is_none());
    assert!(
        card.diagnostics
            .iter()
            .any(|line| line.contains("не найден внешний сайт"))
    );
}

#[test]
fn takes_the_category_from_the_categories_list() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"categories":[{{"id":"1","name":"Пиццерия"}}]}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().category_hint, "Пиццерия");
}

#[test]
fn falls_back_to_the_platform_name_without_a_category() {
    let state = state_with_item(&format!(r#"{{{BUSINESS},"title":"Кафе"}}"#));
    assert_eq!(card_from_state(&state).unwrap().category_hint, "Профиль");
}

#[test]
fn prefers_title_over_short_title() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"title":"Полное","shortTitle":"Короткое"}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().card_title, "Полное");

    let only_short = state_with_item(&format!(r#"{{{BUSINESS},"shortTitle":"Короткое"}}"#));
    assert_eq!(card_from_state(&only_short).unwrap().card_title, "Короткое");
}

#[test]
fn ignores_a_shell_title_and_falls_back_to_the_url() {
    let state = state_with_item(&format!(r#"{{{BUSINESS},"title":"Яндекс карты"}}"#));
    let card = card_from_state(&state).expect("card must parse");

    assert!(card.card_title.contains("25558422006"));
    assert!(!card.card_title.contains("Яндекс карты"));
}

#[test]
fn prefers_full_address_over_address_and_description() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"fullAddress":"Полный адрес","address":"Короткий","description":"Описание"}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().address, "Полный адрес");
}

#[test]
fn collects_social_links_and_counts_them() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"socialLinks":[
            {{"type":"telegram","href":"https://t.me/a"}},
            {{"type":"vkontakte","href":"https://vk.com/b"}}
        ]}}"#
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(card.social_links.len(), 2);
    assert_eq!(card.social_count, 2);
}

#[test]
fn counts_contacts_from_phones_links_and_website() {
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"phones":[{{"number":"+7 999"}},{{"number":"+7 888"}}],
            "businessLinks":[{{"href":"https://b"}}],"urls":["https://site"]}}"#
    ));
    assert_eq!(card_from_state(&state).unwrap().contact_count, 4);
}

#[test]
fn clamps_the_counts_that_have_ceilings() {
    let features: Vec<String> = (0..40)
        .map(|i| format!(r#"{{"id":"feature_{i}","value":true}}"#))
        .collect();
    let state = state_with_item(&format!(
        r#"{{{BUSINESS},"features":[{}]}}"#,
        features.join(",")
    ));
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(card.feature_count, 20);
    assert_eq!(card.photo_count, 0);
    assert_eq!(card.menu_items_count, 0);
}

#[test]
fn survives_a_state_without_any_business_fields() {
    let state = state_with_item(BARE_ITEM);
    let card = card_from_state(&state).expect("card must parse");

    assert_eq!(card.card_title, "Профиль 25558422006");
    assert!(card.address.contains("yandex.ru"));
    assert_eq!(card.description, "Профиль");
    assert_eq!(card.category_hint, "Профиль");
    assert_eq!(card.rating, None);
    assert_eq!(card.review_count, None);
    assert_eq!(card.photo_count, 0);
    assert!(card.social_links.is_empty());
}
