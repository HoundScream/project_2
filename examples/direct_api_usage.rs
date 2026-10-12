//! Shows using `geonetix::scenario_lib` directly from Rust code instead of
//! a JSON scenario file — useful when a check needs branching/logic that
//! doesn't fit the declarative step format. Most scenarios should still be
//! plain JSON under `scenarios/` so both the Rust and Playwright runners
//! can replay them (see `scenarios/README.md`).
//!
//! Run with a server already up (`cargo run` in another terminal):
//!   cargo run --example direct_api_usage

use geonetix::scenario_lib::{PageResponse, Session};

#[tokio::main]
async fn main() {
    let base_url = std::env::var("GEONETIX_TEST_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let session = Session::new(&base_url).expect("failed to build HTTP session");

    let home = session.get("/").await.expect("GET / failed");
    println!("GET / -> {}", home.status);

    // Unique per run: phone/telegram/email all have uniqueness constraints
    // server-side, so fixed literals would only work once.
    let unique = uuid::Uuid::new_v4().simple().to_string();
    let phone_suffix: String = unique
        .chars()
        .take(7)
        .map(|c| ((c as u8 % 10) + b'0') as char)
        .collect();
    let fields = vec![
        ("full_name".to_string(), "Direct API Example".to_string()),
        ("phone".to_string(), format!("+7999{phone_suffix}")),
        (
            "telegram_handle".to_string(),
            format!("@direct_api_{}", &unique[..10]),
        ),
        (
            "email".to_string(),
            format!("direct-api-{unique}@example.com"),
        ),
        ("password".to_string(), "SuperSecret123".to_string()),
    ];
    let registered = session
        .post_form("/register", &fields)
        .await
        .expect("POST /register failed");
    let user_id = PageResponse::capture_after(
        registered.location.as_deref().unwrap_or_default(),
        "/register/success/",
    )
    .expect("registration did not redirect to /register/success/{id} as expected");
    println!("registered user_id={user_id}");

    let cabinet = session
        .get(&format!("/cabinet/{user_id}"))
        .await
        .expect("GET /cabinet/{user_id} failed");
    assert_eq!(
        cabinet.status, 200,
        "cabinet page should be reachable right after registration"
    );
    assert!(
        cabinet
            .visible_text()
            .contains("Бесплатный кабинет Geonetix"),
        "cabinet page should greet the user"
    );
    println!("cabinet reachable and greets the user, as expected");
}
