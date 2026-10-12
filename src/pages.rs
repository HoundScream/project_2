use crate::content::{
    pages::{landing, not_found, pricing, pro_package},
    schema::{ActionLink, ActionVariant, FaqItem, FeatureItem},
    seo::SeoContent,
};
use crate::utils::{
    html_escape, module_eight_stage_label, monitoring_frequency_label, monitoring_status_label,
    platform_label_from_db, report_excerpt, report_format_label, request_status_label,
    subscription_end_label, subscription_plan_label, subscription_status_label,
};
use axum::{http::StatusCode, response::Html};
use geonetix_core::{
    AnalysisStatusRecord, CabinetAnalysisForm, CabinetAnalysisRecord, LoginForm,
    ModuleEightAuditRecord, ModuleEightAuditSummary, ModuleEightForm, ModuleNineReportRecord,
    ModuleNineReportSummary, ModuleTwelveForm, ProMonitoringEvent, ProMonitoringScenario,
    ProPackageForm, RegisterForm, RegisteredUser, SubscriptionFeedback, ValidationIssue,
};

pub fn render_seo_tags(seo: &SeoContent) -> String {
    let keywords = seo
        .keywords
        .iter()
        .map(|keyword| html_escape(keyword))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"<title>{title}</title>
  <meta name="description" content="{description}" />
  <meta name="robots" content="{robots}" />
  <link rel="canonical" href="{canonical_path}" />
  <meta name="keywords" content="{keywords}" />"#,
        title = html_escape(seo.title),
        description = html_escape(seo.description),
        robots = html_escape(seo.robots),
        canonical_path = html_escape(seo.canonical_path),
        keywords = keywords,
    )
}

pub fn render_action_link(
    action: &ActionLink,
    primary_class: &str,
    secondary_class: &str,
) -> String {
    let class_name = match action.variant {
        ActionVariant::Primary => primary_class,
        ActionVariant::Secondary => secondary_class,
    };

    format!(
        r#"<a class="{class_name}" href="{href}">{label}</a>"#,
        class_name = html_escape(class_name),
        href = html_escape(action.href),
        label = html_escape(action.label),
    )
}

pub fn render_action_links(
    actions: &[ActionLink],
    primary_class: &str,
    secondary_class: &str,
) -> String {
    let mut items = String::new();
    for action in actions {
        items.push_str(&render_action_link(action, primary_class, secondary_class));
    }
    items
}

pub fn render_feature_cards(features: &[FeatureItem], class_name: &str) -> String {
    let mut cards = String::new();
    for item in features {
        let title = item
            .title
            .map(|value| format!("<strong>{}</strong>", html_escape(value)))
            .unwrap_or_default();
        cards.push_str(&format!(
            r#"<article class="{class_name}">{title}<p>{description}</p></article>"#,
            class_name = html_escape(class_name),
            title = title,
            description = html_escape(item.description),
        ));
    }
    cards
}

pub fn render_list_items(features: &[FeatureItem]) -> String {
    let mut items = String::new();
    for item in features {
        items.push_str(&format!("<li>{}</li>", html_escape(item.description)));
    }
    items
}

pub fn render_flow_cards(features: &[FeatureItem]) -> String {
    let mut cards = String::new();
    for item in features {
        let title = item.title.unwrap_or_default();
        cards.push_str(&format!(
            r#"<article class="flow-step"><span class="step-label">{title}</span><p>{description}</p></article>"#,
            title = html_escape(title),
            description = html_escape(item.description),
        ));
    }
    cards
}

pub fn render_faq_items(items: &[FaqItem]) -> String {
    let mut faq = String::new();
    for item in items {
        faq.push_str(&format!(
            r#"<article class="card"><strong>{question}</strong><p style="margin-top: 10px;">{answer}</p></article>"#,
            question = html_escape(item.question),
            answer = html_escape(item.answer),
        ));
    }
    faq
}

pub fn render_landing_page() -> String {
    // AI assistants: update visible copy in `content/pages/landing.rs`.
    let feature_cards = render_feature_cards(landing::FEATURES, "card");
    let faq_items = render_faq_items(landing::FAQ);
    let hero_actions = render_action_links(landing::HERO_ACTIONS, "button", "ghost");
    let cta_secondary = landing::CTA
        .secondary_action
        .map(|action| render_action_link(&action, "ghost", "ghost"))
        .unwrap_or_default();
    let footer_links = render_action_links(landing::FOOTER.links, "button", "ghost");

    format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {seo_tags}
  <style>
    :root {{
      --bg-1: #f7f4ec;
      --bg-2: #e8f0e2;
      --ink: #1d2b24;
      --muted: #56655e;
      --card: #ffffffcc;
      --line: #d6dfd0;
      --action: #204f3f;
      --action-hover: #173c30;
      --soft: #eef4ea;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background: radial-gradient(circle at top left, var(--bg-2), var(--bg-1) 70%);
      padding: 24px;
    }}
    main {{
      width: min(980px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 28px;
      backdrop-filter: blur(3px);
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    h1 {{
      font-size: clamp(1.8rem, 4vw, 3rem);
      line-height: 1.02;
      max-width: 12ch;
    }}
    p, li {{
      color: var(--muted);
      line-height: 1.6;
    }}
    .hero {{
      display: grid;
      gap: 14px;
    }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-top: 8px;
    }}
    .button, .ghost {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      text-decoration: none;
      font-size: 1rem;
    }}
    .button {{
      color: #fff;
      background: var(--action);
    }}
    .button:hover {{ background: var(--action-hover); }}
    .ghost {{
      color: var(--action);
      border: 1px solid var(--line);
      background: #fff;
    }}
    form {{
      margin-top: 22px;
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
    }}
    input[type="search"] {{
      flex: 1 1 280px;
      min-height: 48px;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 0 14px;
      font-size: 1rem;
      background: #fff;
    }}
    button {{
      min-height: 48px;
      border: 0;
      border-radius: 12px;
      padding: 0 18px;
      font-size: 1rem;
      color: #fff;
      background: var(--action);
      cursor: pointer;
      transition: background 0.2s ease;
    }}
    button:hover {{ background: var(--action-hover); }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 12px;
    }}
    .card {{
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.94), rgba(238,244,234,0.92));
    }}
    .footer-note {{
      margin-top: 14px;
    }}
    ul {{ padding-left: 18px; }}
  </style>
</head>
<body data-page-id="{page_id}" data-page-route="{page_route}" data-cms-collection="{cms_collection}" data-content-source="content/pages/landing.rs">
  <main>
    <section class="hero">
      <h1>{hero_title}</h1>
      <p>{hero_description}</p>
      <form action="{form_action}" method="{form_method}">
        <input type="{input_type}" name="{field_name}" placeholder="{placeholder}" aria-label="{aria_label}" required />
        <button type="submit">{submit_label}</button>
      </form>
      <div class="actions">
        {hero_actions}
      </div>
    </section>

    <section>
      <h2>{features_title}</h2>
      <div class="grid" style="margin-top: 16px;">
        {feature_cards}
      </div>
    </section>

    <section>
      <h2>{faq_title}</h2>
      <div class="grid" style="margin-top: 16px;">
        {faq_items}
      </div>
    </section>

    <section>
      <h2>{cta_title}</h2>
      <p style="margin-top: 12px;">{cta_description}</p>
      <div class="actions" style="margin-top: 16px;">
        {cta_primary}
        {cta_secondary}
      </div>
      <p class="footer-note">{footer_note}</p>
      <div class="actions" style="margin-top: 12px;">
        {footer_links}
      </div>
    </section>
  </main>
</body>
</html>"#,
        locale = landing::PAGE.metadata.locale,
        seo_tags = render_seo_tags(&landing::PAGE.seo),
        page_id = landing::PAGE.metadata.id,
        page_route = html_escape(landing::PAGE.metadata.route),
        cms_collection = html_escape(landing::PAGE.metadata.cms_collection),
        hero_title = html_escape(landing::HERO.title),
        hero_description = html_escape(landing::HERO.description),
        form_action = html_escape(landing::HERO_FORM.action),
        form_method = html_escape(landing::HERO_FORM.method),
        input_type = html_escape(landing::HERO_FORM.input_type),
        field_name = html_escape(landing::HERO_FORM.field_name),
        placeholder = html_escape(landing::HERO_FORM.placeholder),
        aria_label = html_escape(landing::HERO_FORM.aria_label),
        submit_label = html_escape(landing::HERO_FORM.submit_label),
        hero_actions = hero_actions,
        features_title = html_escape(landing::FEATURES_TITLE),
        feature_cards = feature_cards,
        faq_title = html_escape(landing::FAQ_TITLE),
        faq_items = faq_items,
        cta_title = html_escape(landing::CTA.title),
        cta_description = html_escape(landing::CTA.description),
        cta_primary = render_action_link(&landing::CTA.primary_action, "button", "ghost"),
        cta_secondary = cta_secondary,
        footer_note = html_escape(landing::FOOTER.note),
        footer_links = footer_links,
    )
}

pub fn render_register_page(form: &RegisterForm, errors: &[String]) -> String {
    let error_block = if errors.is_empty() {
        String::new()
    } else {
        let mut items = String::new();
        for error in errors {
            items.push_str(&format!("<li>{}</li>", html_escape(error)));
        }

        format!(
            "<section class=\"notice error\"><h2>Проверьте форму</h2><ul>{items}</ul></section>"
        )
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Регистрация пользователя</title>
  <style>
    :root {{
      --bg: #f4efe5;
      --ink: #1d2b24;
      --muted: #627168;
      --line: #d8dfd5;
      --card: rgba(255, 255, 255, 0.92);
      --accent: #204f3f;
      --accent-soft: #e0ede4;
      --error: #8d2f2f;
      --error-soft: #f9ecec;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #e7efe1, transparent 30%),
        linear-gradient(180deg, #faf6ef 0%, var(--bg) 100%);
    }}
    main {{
      width: min(920px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 28px;
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    .hero {{
      display: grid;
      gap: 12px;
    }}
    .hero p, .hint, li {{
      color: var(--muted);
      line-height: 1.6;
    }}
    .layout {{
      display: grid;
      grid-template-columns: 1.35fr 0.95fr;
      gap: 18px;
    }}
    @media (max-width: 820px) {{
      .layout {{
        grid-template-columns: 1fr;
      }}
    }}
    h1, h2, p, ul {{
      margin: 0;
    }}
    form {{
      display: grid;
      gap: 16px;
    }}
    label {{
      display: grid;
      gap: 8px;
      font-weight: 600;
    }}
    input {{
      min-height: 50px;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 0 14px;
      font-size: 1rem;
      background: #fff;
      color: var(--ink);
    }}
    .hint {{
      font-size: 0.94rem;
    }}
    .notice {{
      display: grid;
      gap: 10px;
    }}
    .notice.error {{
      border-color: #efd0d0;
      background: var(--error-soft);
    }}
    button {{
      min-height: 52px;
      border: 0;
      border-radius: 12px;
      padding: 0 18px;
      font-size: 1rem;
      color: #fff;
      background: var(--accent);
      cursor: pointer;
    }}
    .info-card {{
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.96), rgba(224,237,228,0.9));
    }}
    .info-card + .info-card {{
      margin-top: 12px;
    }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-top: 4px;
    }}
    .text-link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 600;
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <h1>Модуль 4. Регистрация пользователя</h1>
      <p>Создайте аккаунт, чтобы перейти к бесплатному кабинету и в следующем шаге запускать анализ карточек бизнеса из Яндекс Карт, Google Maps и 2ГИС.</p>
      <div class="actions">
        <a class="text-link" href="/">На главную</a>
        <a class="text-link" href="/search?q=%D0%BC%D0%BE%D1%81%D0%BA%D0%B2%D0%B0">Посмотреть демо-анализ</a>
        <a class="text-link" href="/login">Уже есть аккаунт? Войти</a>
      </div>
    </section>

    {error_block}

    <div class="layout">
      <section>
        <form action="/register" method="post">
          <label>
            Имя
            <input type="text" name="full_name" value="{full_name}" placeholder="Например, Анна Петрова" required />
          </label>
          <label>
            Телефон
            <input type="tel" name="phone" value="{phone}" placeholder="+7 999 123-45-67" required />
            <span class="hint">Сохраняем в нормализованном виде, чтобы привязать лимиты и будущую историю отчётов к одному пользователю.</span>
          </label>
          <label>
            Ник в Telegram
            <input type="text" name="telegram_handle" value="{telegram_handle}" placeholder="@geonetix_user" required />
          </label>
          <label>
            Email
            <input type="email" name="email" value="{email}" placeholder="you@example.com" required />
          </label>
          <label>
            Пароль
            <input type="password" name="password" value="" placeholder="Не короче 8 символов" required />
            <span class="hint">Пароль хранится в виде хэша и нужен для дальнейшего входа в кабинет по сессии.</span>
          </label>
          <button type="submit">Зарегистрироваться</button>
        </form>
      </section>

      <section>
        <div class="info-card">
          <h2>Что создаётся вместе с аккаунтом</h2>
          <ul style="margin-top: 12px; padding-left: 18px;">
            <li>Пользователь в PostgreSQL с отдельным ID и статусом аккаунта.</li>
            <li>Бесплатный тариф `free` с лимитом на один анализ.</li>
            <li>Основа для будущего кабинета, истории отчётов и перехода на PRO.</li>
          </ul>
        </div>
        <div class="info-card">
          <h2>Почему это важно для продукта</h2>
          <p style="margin-top: 12px;">Geonetix анализирует карточки бизнеса в гео-сервисах, поэтому нам нужна устойчивая идентификация пользователя до запуска анализа, сохранения результатов и контроля бесплатных лимитов.</p>
        </div>
      </section>
    </div>
  </main>
</body>
</html>"#,
        error_block = error_block,
        full_name = html_escape(form.full_name.trim()),
        phone = html_escape(form.phone.trim()),
        telegram_handle = html_escape(form.telegram_handle.trim()),
        email = html_escape(form.email.trim())
    )
}

pub fn render_login_page(form: &LoginForm, errors: &[String]) -> String {
    let error_block = if errors.is_empty() {
        String::new()
    } else {
        let items = errors
            .iter()
            .map(|error| format!("<li>{}</li>", html_escape(error)))
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<section class=\"notice error\"><h2>Не удалось войти</h2><ul>{items}</ul></section>"
        )
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Вход в Geonetix</title>
  <style>
    :root {{
      --bg: #f5efe4;
      --ink: #1b2a22;
      --muted: #627168;
      --line: #d8dfd5;
      --card: rgba(255,255,255,0.94);
      --accent: #204f3f;
      --accent-soft: #e0ede4;
      --error-soft: #f8ecec;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #e7efe1, transparent 30%),
        linear-gradient(180deg, #fbf7ef 0%, var(--bg) 100%);
    }}
    main {{ width: min(900px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 28px;
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    p, li {{ color: var(--muted); line-height: 1.6; }}
    form {{ display: grid; gap: 16px; }}
    label {{ display: grid; gap: 8px; font-weight: 600; }}
    input {{
      min-height: 50px;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 0 14px;
      font-size: 1rem;
    }}
    button {{
      min-height: 50px;
      border: 0;
      border-radius: 12px;
      background: var(--accent);
      color: #fff;
      font-size: 1rem;
      cursor: pointer;
    }}
    .notice.error {{ background: var(--error-soft); }}
    .actions {{ display: flex; gap: 12px; flex-wrap: wrap; margin-top: 14px; }}
    .text-link {{ color: var(--accent); text-decoration: none; font-weight: 700; }}
    .hint {{ font-size: 0.94rem; }}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>Вход в кабинет Geonetix</h1>
      <p style="margin-top: 12px;">Используйте email или телефон и пароль, чтобы открыть только свой кабинет, историю анализов и модуль управления подпиской.</p>
      <div class="actions">
        <a class="text-link" href="/">На главную</a>
        <a class="text-link" href="/register">Создать новый аккаунт</a>
      </div>
    </section>
    {error_block}
    <section>
      <form action="/login" method="post">
        <label>
          Email или телефон
          <input type="text" name="identifier" value="{identifier}" placeholder="you@example.com или +79991234567" required />
        </label>
        <label>
          Пароль
          <input type="password" name="password" value="" placeholder="Ваш пароль" required />
        </label>
        <span class="hint">После успешного входа сервер создаст сессию и привяжет доступ к вашему аккаунту.</span>
        <button type="submit">Войти</button>
      </form>
    </section>
  </main>
</body>
</html>"#,
        error_block = error_block,
        identifier = html_escape(form.identifier.trim()),
    )
}

pub fn render_register_success_page(
    session_user: &RegisteredUser,
    user: &RegisteredUser,
) -> String {
    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Регистрация завершена</title>
  <style>
    :root {{
      --bg: #f3efe6;
      --ink: #1c2a23;
      --muted: #607067;
      --line: #d8dfd5;
      --card: rgba(255,255,255,0.94);
      --accent: #204f3f;
      --accent-soft: #e0ede4;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #e6efe0, transparent 33%),
        linear-gradient(180deg, #fbf7ef 0%, var(--bg) 100%);
    }}
    main {{
      width: min(860px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 28px;
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    p, li {{ color: var(--muted); line-height: 1.6; }}
    .badge {{
      display: inline-flex;
      align-items: center;
      min-height: 36px;
      padding: 0 12px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 700;
    }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 12px;
      margin-top: 16px;
    }}
    .card {{
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: #fff;
    }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-top: 18px;
    }}
    .button, .ghost {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      text-decoration: none;
    }}
    .button {{
      background: var(--accent);
      color: #fff;
    }}
    .ghost {{
      border: 1px solid var(--line);
      color: var(--accent);
      background: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <section>
      <div class="badge">Аккаунт создан</div>
      <h1 style="margin-top: 14px;">Регистрация завершена для {full_name}</h1>
      <p style="margin-top: 12px;">Пользователь сохранён в базе данных. Следующий шаг по схеме — бесплатный кабинет, где можно вставлять ссылку на карточку бизнеса и запускать анализ.</p>
      <div class="actions">
        <a class="button" href="/cabinet/{user_id}">Открыть бесплатный кабинет</a>
        <a class="ghost" href="/cabinet/{user_id}/subscription">Открыть модуль 13</a>
        <a class="ghost" href="/logout">Выйти</a>
        <a class="ghost" href="/">Вернуться на главную</a>
        <a class="ghost" href="/register">Зарегистрировать ещё одного пользователя</a>
      </div>
    </section>

    <section>
      <h2>Сохранённые данные аккаунта</h2>
      <div class="grid">
        <article class="card">
          <strong>Текущая сессия</strong>
          <p style="margin-top: 8px;">Выполнен вход как {session_full_name}</p>
        </article>
        <article class="card">
          <strong>Телефон</strong>
          <p style="margin-top: 8px;">{phone}</p>
        </article>
        <article class="card">
          <strong>Telegram</strong>
          <p style="margin-top: 8px;">{telegram_handle}</p>
        </article>
        <article class="card">
          <strong>Email</strong>
          <p style="margin-top: 8px;">{email}</p>
        </article>
        <article class="card">
          <strong>Тариф</strong>
          <p style="margin-top: 8px;">{subscription_plan}, статус: {subscription_status}, лимит запусков: {analysis_quota_limit}</p>
        </article>
      </div>
    </section>
  </main>
</body>
</html>"#,
        full_name = html_escape(&user.full_name),
        phone = html_escape(&user.phone),
        telegram_handle = html_escape(&user.telegram_handle),
        email = html_escape(&user.email),
        subscription_plan = html_escape(subscription_plan_label(&user.subscription_plan)),
        subscription_status = html_escape(subscription_status_label(user)),
        analysis_quota_limit = user.analysis_quota_limit,
        session_full_name = html_escape(&session_user.full_name),
        user_id = user.id
    )
}

pub fn render_cabinet_page(
    status: StatusCode,
    user: &RegisteredUser,
    form: &CabinetAnalysisForm,
    issues: &[ValidationIssue],
    records: &[CabinetAnalysisRecord],
) -> (StatusCode, Html<String>) {
    let remaining_analyses = (user.analysis_quota_limit - user.analysis_quota_used).max(0);
    let subscription_plan = subscription_plan_label(&user.subscription_plan);
    let subscription_status = subscription_status_label(user);
    let subscription_end = subscription_end_label(user);
    let show_upgrade_cta = issues.iter().any(|issue| issue.upgrade_to_pro);
    let error_block = if issues.is_empty() {
        String::new()
    } else {
        let mut items = String::new();
        for issue in issues {
            items.push_str(&format!(
                r#"<article class="validation-item">
                  <strong>{title}</strong>
                  <p>{detail}</p>
                </article>"#,
                title = html_escape(&issue.title),
                detail = html_escape(&issue.detail),
            ));
        }

        let upgrade_block = if show_upgrade_cta {
            format!(
                r#"<div class="validation-upgrade">
                  <strong>Следующий шаг: модуль 13. Управление подпиской</strong>
                  <p>Лимит или срок подписки закончился, поэтому дальше ведём пользователя в кабинетный сценарий продления без тупиковой ошибки.</p>
                  <a class="pro-button" href="/cabinet/{user_id}/subscription">Открыть модуль 13</a>
                  <p>Внутри доступны продление лимита, смена тарифа, автопродление и отмена.</p>
                </div>"#,
                user_id = user.id
            )
        } else {
            String::new()
        };

        format!(
            r#"<section class="notice error validation-module">
              <p class="module-kicker">Модуль 2.1</p>
              <h2>Validation / Error Handler</h2>
              <p class="validation-lead">После ввода ссылки система проверяет лимит, тип тарифа, доступность карточки, отклик источника и наличие такой карточки в истории текущего пользователя.</p>
              <div class="validation-list">{items}</div>
              {upgrade_block}
            </section>"#
        )
    };

    let mut request_cards = String::new();
    for record in records {
        let platform_label = platform_label_from_db(&record.source_platform);
        let profile_title = record
            .profile_title
            .as_deref()
            .map(html_escape)
            .unwrap_or_else(|| "Название подтянем на следующем этапе ingestion".to_string());
        let profile_address = record
            .profile_address
            .as_deref()
            .map(html_escape)
            .unwrap_or_else(|| "Адрес пока не разобран".to_string());

        request_cards.push_str(&format!(
            r#"<article class="request-card">
              <div class="request-top">
                <strong>{platform_label}</strong>
                <span class="status">{request_status}</span>
              </div>
              <p class="request-link"><a href="{canonical_url}" target="_blank" rel="noreferrer">{canonical_url}</a></p>
              <p>{profile_title}</p>
              <p>{profile_address}</p>
              <p class="request-meta">Запрос: {request_id} · тип: {analysis_kind} · создан: {requested_at}</p>
              <p class="request-meta"><a class="text-link" href="/cabinet/{user_id}/analysis/{request_id}">Открыть статус анализа</a></p>
            </article>"#,
            platform_label = html_escape(platform_label),
            request_status = html_escape(request_status_label(&record.request_status)),
            canonical_url = html_escape(&record.canonical_url),
            profile_title = profile_title,
            profile_address = profile_address,
            request_id = record.request_id,
            analysis_kind = html_escape(&record.analysis_kind),
            requested_at = html_escape(&record.requested_at),
            user_id = user.id,
        ));
    }

    if request_cards.is_empty() {
        request_cards.push_str(
            "<article class=\"request-card empty\"><p>Пока нет сохранённых запросов. Вставьте ссылку на карточку Яндекс / Google / 2GIS, и мы создадим реальную запись анализа в базе.</p></article>",
        );
    }

    let html = format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Бесплатный кабинет Geonetix</title>
  <style>
    :root {{
      --bg: #f4efe6;
      --ink: #1d2b24;
      --muted: #607067;
      --line: #d7dfd4;
      --card: rgba(255,255,255,0.93);
      --accent: #204f3f;
      --accent-soft: #e1ede5;
      --error: #8f3232;
      --error-soft: #fbefef;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #e6efe0, transparent 32%),
        linear-gradient(180deg, #faf7f0 0%, var(--bg) 100%);
    }}
    main {{
      width: min(1080px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 28px;
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    p, li {{ color: var(--muted); line-height: 1.6; }}
    .hero {{
      display: grid;
      gap: 14px;
    }}
    .hero-top {{
      display: flex;
      justify-content: space-between;
      gap: 16px;
      align-items: start;
      flex-wrap: wrap;
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      min-height: 38px;
      padding: 0 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 700;
    }}
    .layout {{
      display: grid;
      grid-template-columns: 1.2fr 0.8fr;
      gap: 18px;
    }}
    .stats {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 12px;
    }}
    .metric {{
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.96), rgba(225,237,229,0.88));
    }}
    .metric strong {{
      display: block;
      font-size: 1.8rem;
      color: var(--accent);
      margin-bottom: 6px;
    }}
    form {{
      display: grid;
      gap: 16px;
    }}
    label {{
      display: grid;
      gap: 8px;
      font-weight: 600;
    }}
    input {{
      min-height: 52px;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 0 14px;
      font-size: 1rem;
      color: var(--ink);
      background: #fff;
    }}
    button {{
      min-height: 52px;
      border: 0;
      border-radius: 12px;
      background: var(--accent);
      color: #fff;
      font-size: 1rem;
      cursor: pointer;
    }}
    .hint {{
      font-size: 0.94rem;
    }}
    .notice.error {{
      background: var(--error-soft);
      border-color: #efd2d2;
    }}
    .module-kicker {{
      font-size: 0.82rem;
      font-weight: 800;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--error);
    }}
    .validation-module {{
      display: grid;
      gap: 14px;
    }}
    .validation-lead {{
      color: #6f4b4b;
    }}
    .validation-list {{
      display: grid;
      gap: 12px;
    }}
    .validation-item {{
      border: 1px solid #efd2d2;
      border-radius: 16px;
      padding: 16px;
      background: rgba(255, 255, 255, 0.78);
    }}
    .validation-item strong {{
      display: block;
      margin-bottom: 6px;
      color: var(--error);
    }}
    .validation-upgrade {{
      display: grid;
      gap: 10px;
      padding: 18px;
      border-radius: 16px;
      border: 1px solid #e1c8b0;
      background: linear-gradient(135deg, #fffaf1 0%, #f7ead5 100%);
    }}
    .pro-button {{
      display: inline-flex;
      justify-content: center;
      align-items: center;
      width: fit-content;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      background: #9c5c16;
      color: #fff;
      text-decoration: none;
      font-weight: 700;
    }}
    .request-list {{
      display: grid;
      gap: 12px;
      margin-top: 16px;
    }}
    .request-card {{
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: #fff;
      display: grid;
      gap: 8px;
    }}
    .request-card.empty {{
      background: #fcfbf8;
    }}
    .request-top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      flex-wrap: wrap;
    }}
    .status {{
      color: var(--accent);
      background: var(--accent-soft);
      border-radius: 999px;
      padding: 6px 10px;
      font-size: 0.92rem;
      font-weight: 700;
    }}
    .request-link a {{
      color: var(--accent);
      word-break: break-all;
    }}
    .request-meta {{
      font-size: 0.92rem;
    }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }}
    .subscription-summary {{
      margin-top: 16px;
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.98), rgba(225,237,229,0.9));
      display: grid;
      gap: 10px;
    }}
    .text-link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 600;
    }}
    @media (max-width: 880px) {{
      .layout {{
        grid-template-columns: 1fr;
      }}
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <div class="hero-top">
        <div>
          <h1>Модуль 4.1. Бесплатный кабинет</h1>
          <p style="margin-top: 10px;">Кабинет принимает реальные ссылки на карточки бизнеса из Яндекс Карт, Google Maps и 2GIS. Перед созданием запроса теперь отрабатывает отдельный модуль `2.1 Validation / Error Handler`, чтобы пользователь сразу видел причину отказа и следующий шаг.</p>
        </div>
        <div class="badge">Пользователь: {full_name}</div>
      </div>
      <div class="actions">
        <a class="text-link" href="/">На главную</a>
        <a class="text-link" href="/register/success/{user_id}">Вернуться к завершению регистрации</a>
        <a class="text-link" href="/cabinet/{user_id}/subscription">Открыть модуль 13</a>
        <a class="text-link" href="/logout">Выйти</a>
      </div>
    </section>

    {error_block}

    <div class="layout">
      <section>
        <h2>Запустить бесплатный анализ</h2>
        <p style="margin-top: 12px;">Поддерживаем живые ссылки на карточки в `yandex.ru/maps`, `maps.google.*` и `2gis.ru`. До создания `analysis_requests` система проверяет формат ссылки, лимит текущего тарифа, доступность источника и отсутствие дубля в истории.</p>
        <form action="/cabinet/{user_id}" method="post" style="margin-top: 18px;">
          <label>
            Ссылка на карточку бизнеса
            <input type="url" name="source_url" value="{source_url}" placeholder="https://yandex.ru/maps/org/... или https://maps.google.com/... или https://2gis.ru/..." required />
          </label>
          <span class="hint">Вставьте полную ссылку из адресной строки, начиная с `http://` или `https://`. Короткие, обрезанные или повреждённые ссылки проверку не пройдут. Для `free`-тарифа действует ограниченный лимит, а после успешной проверки вы перейдёте на экран статуса анализа.</span>
          <button type="submit">Анализировать</button>
        </form>
      </section>

      <section>
        <h2>Лимиты и подписка</h2>
        <div class="stats" style="margin-top: 16px;">
          <article class="metric">
            <strong>{analysis_quota_limit}</strong>
            <span>лимит запусков в пакете</span>
          </article>
          <article class="metric">
            <strong>{analysis_quota_used}</strong>
            <span>уже использовано</span>
          </article>
          <article class="metric">
            <strong>{remaining_analyses}</strong>
            <span>ещё доступно</span>
          </article>
        </div>
        <div class="subscription-summary">
          <p><strong>Модуль 13:</strong> {subscription_plan}</p>
          <p><strong>Статус:</strong> {subscription_status}</p>
          <p><strong>Дата окончания:</strong> {subscription_end}</p>
          <a class="text-link" href="/cabinet/{user_id}/subscription">Перейти к управлению подпиской</a>
          <p>Оплата в тестовом контуре пока симулируется внутри продукта, без внешнего биллинга.</p>
        </div>
      </section>
    </div>

    <section>
      <h2>История запросов</h2>
      <div class="request-list">{request_cards}</div>
    </section>
  </main>
</body>
</html>"#,
        full_name = html_escape(&user.full_name),
        user_id = user.id,
        source_url = html_escape(form.source_url.trim()),
        analysis_quota_limit = user.analysis_quota_limit,
        analysis_quota_used = user.analysis_quota_used,
        remaining_analyses = remaining_analyses,
        subscription_plan = html_escape(subscription_plan),
        subscription_status = html_escape(subscription_status),
        subscription_end = html_escape(&subscription_end),
        error_block = error_block,
        request_cards = request_cards,
    );

    (status, Html(html))
}

pub fn render_subscription_page(
    user: &RegisteredUser,
    feedback: Option<&SubscriptionFeedback>,
) -> String {
    let remaining_analyses = (user.analysis_quota_limit - user.analysis_quota_used).max(0);
    let feedback_block = feedback.map_or_else(String::new, |feedback| {
        let class_name = if feedback.is_error {
            "feedback error"
        } else {
            "feedback"
        };
        format!(
            r#"<section class="{class_name}">
              <strong>{title}</strong>
              <p>{detail}</p>
            </section>"#,
            class_name = class_name,
            title = html_escape(&feedback.title),
            detail = html_escape(&feedback.detail),
        )
    });

    let auto_renew_label = if user.subscription_auto_renew {
        "включено"
    } else {
        "выключено"
    };
    let cancel_label = if user.subscription_cancel_at_period_end {
        "Подписка уже поставлена на отмену в конце периода."
    } else {
        "Подписка не отменена и может продлеваться дальше."
    };

    let renewal_actions = if user.subscription_plan == "pro_10" {
        r#"<form action="" method="post">
            <input type="hidden" name="action" value="renew_pro_10" />
            <button type="submit">Продлить PRO 10</button>
          </form>
          <form action="" method="post">
            <input type="hidden" name="action" value="change_to_pro_3" />
            <button class="ghost" type="submit">Сменить на PRO 3</button>
          </form>"#
            .to_string()
    } else {
        r#"<form action="" method="post">
            <input type="hidden" name="action" value="renew_pro_3" />
            <button type="submit">Продлить PRO 3</button>
          </form>
          <form action="" method="post">
            <input type="hidden" name="action" value="change_to_pro_10" />
            <button class="ghost" type="submit">Сменить на PRO 10</button>
          </form>"#
            .to_string()
    };

    let primary_actions = if user.account_tier == "free" {
        r#"<form action="" method="post">
            <input type="hidden" name="action" value="change_to_pro_3" />
            <button type="submit">Подключить PRO 3</button>
          </form>
          <form action="" method="post">
            <input type="hidden" name="action" value="change_to_pro_10" />
            <button class="ghost" type="submit">Подключить PRO 10</button>
          </form>"#
            .to_string()
    } else {
        format!(
            r#"{renewal_actions}
          <form action="" method="post">
            <input type="hidden" name="action" value="toggle_auto_renew" />
            <button class="ghost" type="submit">Автопродление: {auto_renew_label}</button>
          </form>
          <form action="" method="post">
            <input type="hidden" name="action" value="cancel_subscription" />
            <button class="danger" type="submit">Отменить подписку</button>
          </form>"#,
            renewal_actions = renewal_actions,
            auto_renew_label = html_escape(auto_renew_label),
        )
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 13. Управление подпиской</title>
  <style>
    :root {{
      --bg: #f2efe8;
      --ink: #19251f;
      --muted: #637269;
      --line: #d7ddd2;
      --card: rgba(255,255,255,0.95);
      --accent: #245a46;
      --accent-soft: #e0eee6;
      --warning: #9a6218;
      --warning-soft: #fbf1df;
      --error: #8b3535;
      --error-soft: #fbefef;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #e4eee7, transparent 28%),
        linear-gradient(180deg, #faf8f2 0%, var(--bg) 100%);
    }}
    main {{ width: min(1080px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 22px;
      padding: 28px;
      box-shadow: 0 16px 40px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .hero, .grid, .actions, .action-grid {{ display: grid; gap: 14px; }}
    .hero-top, .card-top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: start;
      flex-wrap: wrap;
    }}
    .pill {{
      display: inline-flex;
      align-items: center;
      min-height: 38px;
      padding: 0 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      font-size: 0.8rem;
    }}
    .grid {{
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }}
    .metric, .action-card {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.98), rgba(224,238,230,0.9));
    }}
    .metric strong {{
      display: block;
      margin-bottom: 8px;
      font-size: 1.7rem;
      color: var(--accent);
    }}
    .feedback {{
      background: var(--warning-soft);
      border-color: #ebd8b8;
    }}
    .feedback.error {{
      background: var(--error-soft);
      border-color: #eed4d4;
    }}
    .action-grid {{
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }}
    form {{
      display: grid;
      gap: 10px;
    }}
    button, .link-button {{
      min-height: 48px;
      border-radius: 12px;
      border: 0;
      padding: 0 16px;
      font-size: 0.98rem;
      font-weight: 700;
      cursor: pointer;
      text-decoration: none;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }}
    button {{
      background: var(--accent);
      color: #fff;
    }}
    button.ghost {{
      background: #fff;
      color: var(--accent);
      border: 1px solid var(--line);
    }}
    button.danger {{
      background: #fff4f4;
      color: var(--error);
      border: 1px solid #efcaca;
    }}
    .nav {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }}
    .text-link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 700;
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <div class="hero-top">
        <div>
          <span class="pill">Модуль 13</span>
          <h1>Управление подпиской в личном кабинете</h1>
          <p>Экран собирает текущий тариф, дату окончания, автопродление, смену пакета и отмену подписки в одном месте без выхода из кабинета.</p>
        </div>
        <div class="pill">{subscription_status}</div>
      </div>
      <div class="nav">
        <a class="text-link" href="/cabinet/{user_id}">Назад в кабинет</a>
        <a class="text-link" href="/register/success/{user_id}">К завершению регистрации</a>
        <a class="text-link" href="/logout">Выйти</a>
      </div>
    </section>

    {feedback_block}

    <section>
      <div class="grid">
        <article class="metric">
          <strong>{subscription_plan}</strong>
          <p>Текущий тариф пользователя.</p>
        </article>
        <article class="metric">
          <strong>{subscription_end}</strong>
          <p>Дата окончания текущего периода.</p>
        </article>
        <article class="metric">
          <strong>{auto_renew_label}</strong>
          <p>Автопродление подписки.</p>
        </article>
        <article class="metric">
          <strong>{remaining_analyses}</strong>
          <p>Осталось запусков в текущем лимите.</p>
        </article>
      </div>
      <p style="margin-top: 16px;">{cancel_label}</p>
      <p style="margin-top: 10px;">Тестовый режим оплаты: кнопки ниже симулируют успешную оплату и смену плана без реального платёжного шлюза.</p>
    </section>

    <section>
      <h2>Действия по подписке</h2>
      <div class="action-grid" style="margin-top: 16px;">
        <article class="action-card">
          <div class="card-top">
            <strong>Продлить лимит</strong>
          </div>
          <p>Подходит, когда лимит пакета уже израсходован или нужно заранее продлить период.</p>
          <div class="actions">{primary_actions}</div>
        </article>
        <article class="action-card">
          <div class="card-top">
            <strong>Что даёт модуль 13</strong>
          </div>
          <p>Пользователь не теряется между внешней оплатой и кабинетом: вся логика продления, смены тарифа и отключения автосписания находится прямо внутри продукта.</p>
        </article>
      </div>
    </section>
  </main>
</body>
</html>"#,
        subscription_status = html_escape(subscription_status_label(user)),
        user_id = user.id,
        feedback_block = feedback_block,
        subscription_plan = html_escape(subscription_plan_label(&user.subscription_plan)),
        subscription_end = html_escape(&subscription_end_label(user)),
        auto_renew_label = html_escape(auto_renew_label),
        remaining_analyses = remaining_analyses,
        cancel_label = html_escape(cancel_label),
        primary_actions = primary_actions,
    )
}

pub fn render_analysis_status_page(
    user: &RegisteredUser,
    analysis: &AnalysisStatusRecord,
) -> String {
    let (
        stage_index,
        progress_percent,
        status_title,
        status_note,
        status_detail,
        final_note_detail,
        is_final,
    ) = analysis_view_model(analysis);
    let platform_label = platform_label_from_db(&analysis.source_platform);
    let card_title = analysis
        .profile_title
        .as_deref()
        .unwrap_or("Карточка организации");
    let card_address = analysis.profile_address.as_deref().unwrap_or("");
    let refresh_tag = if is_final {
        String::new()
    } else {
        "<meta http-equiv=\"refresh\" content=\"2\">".to_string()
    };
    let rating_label = free_rating_label(analysis);
    let summary = report_excerpt(analysis.report_summary.as_deref());
    let strengths = report_excerpt(analysis.report_strengths.as_deref());
    let risks = report_excerpt(analysis.report_risks.as_deref());
    let stage_titles = [
        "Проверяем ссылку",
        "Получаем данные",
        "Анализируем отзывы",
        "Сравниваем конкурентов",
        "Формируем отчёт",
        "Сохраняем результат",
    ];
    let stage_steps = stage_titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let idx = i + 1;
            let (dot_class, label_class) = if idx < stage_index {
                ("step-dot done", "step-label done")
            } else if idx == stage_index {
                ("step-dot active", "step-label active")
            } else {
                ("step-dot", "step-label")
            };
            format!(
                r#"<li class="step">
                  <span class="{dot_class}"></span>
                  <span class="{label_class}">{title}</span>
                </li>"#,
                dot_class = dot_class,
                label_class = label_class,
                title = html_escape(title),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {refresh_tag}
  <title>Статус анализа — Geonetix</title>
  <style>
    :root {{
      --bg: #eef2f5;
      --ink: #16232b;
      --muted: #5c6d76;
      --line: #d6dde3;
      --card: rgba(255,255,255,0.97);
      --accent: #245d78;
      --accent-soft: #e2eef4;
      --warm-soft: #fdf3e3;
      --error: #a24132;
      --error-soft: #fae8e5;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0; min-height: 100vh; padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background: radial-gradient(circle at top right, #dfeaf1, transparent 24%),
        linear-gradient(180deg, #fafcfd 0%, var(--bg) 100%);
    }}
    main {{ width: min(860px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card); border: 1px solid var(--line);
      border-radius: 24px; padding: 28px;
      box-shadow: 0 18px 45px rgba(22,35,30,0.08);
    }}
    h1 {{ margin: 0 0 6px; font-size: 1.35rem; }}
    h2 {{ margin: 0 0 14px; font-size: 1.05rem; color: var(--accent); }}
    p {{ margin: 0 0 10px; line-height: 1.6; color: var(--muted); }}
    .progress-bar {{ height: 8px; background: var(--line); border-radius: 4px; overflow: hidden; margin: 12px 0; }}
    .progress-fill {{ height: 100%; background: var(--accent); border-radius: 4px; transition: width .4s; }}
    .step-list {{ list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }}
    .step {{ display: flex; align-items: center; gap: 10px; }}
    .step-dot {{ width: 12px; height: 12px; border-radius: 50%; background: var(--line); flex-shrink: 0; }}
    .step-dot.done {{ background: var(--accent); }}
    .step-dot.active {{ background: #e8a32a; }}
    .step-label {{ font-size: .9rem; color: var(--muted); }}
    .step-label.done {{ color: var(--accent); }}
    .step-label.active {{ color: #916010; font-weight: 600; }}
    .report-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 14px; }}
    .report-card {{ background: var(--accent-soft); border-radius: 14px; padding: 16px; }}
    .report-card strong {{ display: block; margin-bottom: 6px; font-size: .85rem; color: var(--accent); }}
    .back-link {{ display: inline-block; margin-top: 4px; color: var(--accent); text-decoration: none; font-size: .9rem; }}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>{card_title}</h1>
      <p style="margin:0;font-size:.9rem;">{platform_label} · {canonical_url}{address_line}</p>
    </section>
    <section>
      <h2>{status_title}</h2>
      <p>{status_note}</p>
      <div class="progress-bar"><div class="progress-fill" style="width:{progress_percent}%"></div></div>
      <p style="font-size:.85rem;margin-bottom:14px;">{status_detail}</p>
      <ul class="step-list">{stage_steps}</ul>
    </section>
    <section>
      <h2>{rating_label}</h2>
      <div class="report-grid">
        <div class="report-card">
          <strong>Краткое резюме</strong>
          <p style="margin:0;font-size:.9rem;">{summary}</p>
        </div>
        <div class="report-card">
          <strong>Сильные стороны</strong>
          <p style="margin:0;font-size:.9rem;">{strengths}</p>
        </div>
        <div class="report-card">
          <strong>Зоны риска</strong>
          <p style="margin:0;font-size:.9rem;">{risks}</p>
        </div>
      </div>
    </section>
    <section style="background:{final_note_bg};border-color:{final_note_border};">
      <h2 style="color:{final_note_ink};">{final_note_title}</h2>
      <p style="color:{final_note_ink};">{final_note_detail}</p>
      <a class="back-link" href="/cabinet/{user_id}">← Вернуться в кабинет</a>
    </section>
  </main>
</body>
</html>"#,
        refresh_tag = refresh_tag,
        card_title = html_escape(card_title),
        platform_label = html_escape(platform_label),
        canonical_url = html_escape(&analysis.canonical_url),
        address_line = if card_address.is_empty() {
            String::new()
        } else {
            format!(" · {}", html_escape(card_address))
        },
        status_title = html_escape(status_title),
        status_note = html_escape(status_note),
        progress_percent = progress_percent,
        status_detail = html_escape(status_detail),
        stage_steps = stage_steps,
        rating_label = html_escape(rating_label),
        summary = html_escape(&summary),
        strengths = html_escape(&strengths),
        risks = html_escape(&risks),
        final_note_bg = final_note_bg(analysis),
        final_note_border = final_note_border(analysis),
        final_note_ink = final_note_ink(analysis),
        final_note_title = html_escape(final_note_title(analysis)),
        final_note_detail = html_escape(final_note_detail),
        user_id = user.id,
    )
}

pub fn analysis_view_model(
    analysis: &AnalysisStatusRecord,
) -> (usize, usize, &str, &str, &str, &str, bool) {
    match analysis.request_status.as_str() {
        "completed" => (
            6,
            100,
            "Анализ завершён",
            "Результат автоматически сохранён в кабинете пользователя.",
            "Все этапы завершены, можно вернуться к истории запросов.",
            "Отдельная кнопка сохранения не нужна: отчёт уже прикреплён к запросу в кабинете.",
            true,
        ),
        "failed" => (
            analysis.stage_index.unwrap_or(1).max(1) as usize,
            analysis.progress_percent.unwrap_or(24).max(0) as usize,
            "Не удалось завершить анализ",
            "Фоновый движок остановился на одном из этапов обработки.",
            analysis
                .error_message
                .as_deref()
                .unwrap_or("История запроса сохранена, но результат не был собран полностью."),
            "Из этого экрана недоступны изменение ссылки и повторный запуск. Следующий шаг контролируется через кабинет.",
            true,
        ),
        _ => (
            analysis.stage_index.unwrap_or(1).max(1) as usize,
            analysis.progress_percent.unwrap_or(5).max(0) as usize,
            analysis
                .status_message
                .as_deref()
                .unwrap_or("Анализируем данные"),
            "Пожалуйста, подождите. Фоновый движок проходит шаги анализа и сохраняет результат автоматически.",
            analysis
                .status_detail
                .as_deref()
                .unwrap_or("Готовим запрос к обработке"),
            "Сохраняем результат автоматически, поэтому дополнительных действий во время анализа не требуется.",
            false,
        ),
    }
}

fn free_rating_label(analysis: &AnalysisStatusRecord) -> &'static str {
    if analysis.request_status == "completed" {
        "Базовая оценка собрана"
    } else {
        "Оценка готовится"
    }
}

pub fn final_note_title(analysis: &AnalysisStatusRecord) -> &'static str {
    match analysis.request_status.as_str() {
        "completed" => "Результат сохранён",
        "failed" => "Анализ не завершён",
        _ => "Автосохранение включено",
    }
}

pub fn final_note_bg(analysis: &AnalysisStatusRecord) -> &'static str {
    match analysis.request_status.as_str() {
        "completed" => "var(--accent-soft)",
        "failed" => "var(--error-soft)",
        _ => "var(--warm-soft)",
    }
}

pub fn final_note_ink(analysis: &AnalysisStatusRecord) -> &'static str {
    match analysis.request_status.as_str() {
        "failed" => "var(--error)",
        _ => "var(--ink)",
    }
}

pub fn final_note_border(analysis: &AnalysisStatusRecord) -> &'static str {
    match analysis.request_status.as_str() {
        "completed" => "#b7d4c1",
        "failed" => "#e2c3c3",
        _ => "#ead9be",
    }
}

pub fn render_not_found_page(title: &str, text: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {seo_tags}
  <title>{title}</title>
</head>
<body data-page-id="{page_id}" data-page-route="{page_route}" data-cms-collection="{cms_collection}" data-content-source="content/pages/not_found.rs" style="margin:0;padding:32px;font-family:'Segoe UI','Helvetica Neue',sans-serif;background:#f7f4ec;color:#1d2b24;">
  <main style="max-width:720px;margin:0 auto;background:#fff;border:1px solid #d8dfd5;border-radius:18px;padding:28px;">
    <h1 style="margin:0 0 12px;">{title}</h1>
    <p style="margin:0 0 18px;color:#617168;line-height:1.6;">{text}</p>
    <a href="{home_href}" style="color:#204f3f;">{home_label}</a>
    <p style="margin:18px 0 0;color:#617168;line-height:1.6;">{footer_note}</p>
  </main>
</body>
</html>"#,
        locale = not_found::PAGE.metadata.locale,
        seo_tags = render_seo_tags(&not_found::PAGE.seo),
        page_id = not_found::PAGE.metadata.id,
        page_route = html_escape(not_found::PAGE.metadata.route),
        cms_collection = html_escape(not_found::PAGE.metadata.cms_collection),
        title = html_escape(title),
        text = html_escape(text),
        home_href = html_escape(not_found::HOME_LINK.href),
        home_label = html_escape(not_found::HOME_LINK.label),
        footer_note = html_escape(not_found::FOOTER.note),
    )
}

pub fn render_pricing_page() -> String {
    let pricing_features = render_list_items(pricing::FEATURES);
    let flow_cards = render_flow_cards(pricing::FLOW);

    format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {seo_tags}
  <style>
    :root {{
      --bg: #f5efe5;
      --ink: #1e2a24;
      --muted: #617168;
      --line: #ddd5c8;
      --card: rgba(255,255,255,0.95);
      --accent: #9c5c16;
      --accent-soft: #f6e8d4;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #f5dfb9, transparent 28%),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    main {{
      width: min(760px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 22px;
      padding: 28px;
      box-shadow: 0 18px 44px rgba(42, 36, 28, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    p, li {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      font-size: 0.8rem;
    }}
    .price-card {{
      display: grid;
      gap: 14px;
    }}
    .flow {{
      display: grid;
      gap: 12px;
    }}
    .flow-step {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fcfaf6;
      display: grid;
      gap: 8px;
    }}
    .step-label {{
      font-size: 0.82rem;
      font-weight: 800;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--accent);
    }}
    .price {{
      font-size: 2.2rem;
      font-weight: 800;
      color: var(--accent);
    }}
    .cta {{
      display: inline-flex;
      justify-content: center;
      align-items: center;
      width: fit-content;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 700;
    }}
    .back-link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 700;
    }}
  </style>
</head>
<body data-page-id="{page_id}" data-page-route="{page_route}" data-cms-collection="{cms_collection}" data-content-source="content/pages/pricing.rs">
  <main>
    <section class="price-card">
      <span class="pill">{eyebrow}</span>
      <h1>{hero_title}</h1>
      <p>{hero_description}</p>
      <p class="price">{price_label}</p>
      <ul style="padding-left: 18px;">
        {pricing_features}
      </ul>
      <a class="cta" href="{cta_href}">{cta_label}</a>
    </section>

    <section>
      <h2>{flow_title}</h2>
      <div class="flow" style="margin-top: 14px;">
        {flow_cards}
      </div>
      <p style="margin-top: 14px;"><a class="back-link" href="{back_href}">{back_label}</a></p>
      <p style="margin-top: 12px;">{footer_note}</p>
    </section>
  </main>
</body>
</html>"#,
        locale = pricing::PAGE.metadata.locale,
        seo_tags = render_seo_tags(&pricing::PAGE.seo),
        page_id = pricing::PAGE.metadata.id,
        page_route = html_escape(pricing::PAGE.metadata.route),
        cms_collection = html_escape(pricing::PAGE.metadata.cms_collection),
        eyebrow = html_escape(pricing::HERO.eyebrow.unwrap_or("")),
        hero_title = html_escape(pricing::HERO.title),
        hero_description = html_escape(pricing::HERO.description),
        price_label = html_escape(pricing::PRICE_LABEL),
        pricing_features = pricing_features,
        cta_href = html_escape(pricing::CTA.primary_action.href),
        cta_label = html_escape(pricing::CTA.primary_action.label),
        flow_title = html_escape(pricing::FLOW_TITLE),
        flow_cards = flow_cards,
        back_href = html_escape(pricing::FOOTER.links[0].href),
        back_label = html_escape(pricing::FOOTER.links[0].label),
        footer_note = html_escape(pricing::FOOTER.note),
    )
}

pub fn render_pro_package_page(form: &ProPackageForm, issues: &[String]) -> String {
    let mut issue_items = String::new();
    for issue in issues {
        issue_items.push_str(&format!("<li>{}</li>", html_escape(issue)));
    }

    let status_block = format!(
        r#"<section class="status-card{success_class}">
          <h2>{status_title}</h2>
          <p>{status_text}</p>
          {issue_list}
        </section>"#,
        success_class = if issues.is_empty() { " success" } else { "" },
        status_title = html_escape(pro_package::STATUS.title),
        status_text = if issues.is_empty() {
            html_escape(pro_package::STATUS.success_text)
        } else {
            html_escape(pro_package::STATUS.error_text)
        },
        issue_list = if issue_items.is_empty() {
            format!(
                r#"<div class="summary">
                <p><strong>{}</strong> {}</p>
              </div>"#,
                html_escape(pro_package::STATUS.next_step_title),
                html_escape(pro_package::STATUS.next_step_description)
            )
        } else {
            format!(r#"<ul style="margin-top: 14px; padding-left: 18px;">{issue_items}</ul>"#)
        }
    );

    let benefits = render_flow_cards(pro_package::BENEFITS);

    format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {seo_tags}
  <style>
    :root {{
      --bg: #f5efe5;
      --ink: #1e2a24;
      --muted: #617168;
      --line: #ddd5c8;
      --card: rgba(255,255,255,0.95);
      --accent: #9c5c16;
      --accent-soft: #f6e8d4;
      --success: #215944;
      --success-soft: #e8f2ed;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #f5dfb9, transparent 28%),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    main {{
      width: min(920px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 22px;
      padding: 28px;
      box-shadow: 0 18px 44px rgba(42, 36, 28, 0.08);
    }}
    h1, h2, p, ul {{ margin: 0; }}
    p, li {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      font-size: 0.8rem;
      width: fit-content;
    }}
    .hero {{
      display: grid;
      gap: 14px;
    }}
    form {{
      display: grid;
      gap: 16px;
      margin-top: 8px;
    }}
    label {{
      display: grid;
      gap: 8px;
      font-weight: 600;
    }}
    input, select {{
      min-height: 52px;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 0 14px;
      font-size: 1rem;
      color: var(--ink);
      background: #fff;
    }}
    button {{
      min-height: 52px;
      border: 0;
      border-radius: 12px;
      padding: 0 18px;
      font-size: 1rem;
      color: #fff;
      background: var(--accent);
      cursor: pointer;
      width: fit-content;
    }}
    .status-card {{
      display: grid;
      gap: 12px;
      background: #fcfaf6;
    }}
    .status-card.success {{
      border-color: #cfe0d5;
      background: linear-gradient(180deg, #fcfdfb 0%, var(--success-soft) 100%);
    }}
    .summary {{
      display: grid;
      gap: 8px;
      padding: 16px;
      border-radius: 16px;
      background: rgba(255, 255, 255, 0.76);
      border: 1px solid #d8e4dc;
    }}
    .back-link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 700;
    }}
  </style>
</head>
<body data-page-id="{page_id}" data-page-route="{page_route}" data-cms-collection="{cms_collection}" data-content-source="content/pages/pro_package.rs">
  <main>
    <section class="hero">
      <span class="pill">{eyebrow}</span>
      <h1>{hero_title}</h1>
      <p>{hero_description}</p>

      <form action="/pricing/pro" method="post">
        <label>
          {query_label}
          <input type="{query_input_type}" name="{query_name}" value="{business_query}" placeholder="{query_placeholder}" required />
        </label>
        <label>
          {category_label}
          <select name="{category_name}" required>
            {category_options}
          </select>
        </label>
        <button type="submit">{submit_label}</button>
      </form>
    </section>

    {status_block}

    <section>
      <h2>Почему контент вынесен отдельно</h2>
      <div class="status-card" style="margin-top: 14px;">
        {benefits}
      </div>
      <p style="margin-top: 14px;"><a class="back-link" href="{back_href}">{back_label}</a></p>
      <p style="margin-top: 12px;">{footer_note}</p>
    </section>
  </main>
</body>
</html>"#,
        locale = pro_package::PAGE.metadata.locale,
        seo_tags = render_seo_tags(&pro_package::PAGE.seo),
        page_id = pro_package::PAGE.metadata.id,
        page_route = html_escape(pro_package::PAGE.metadata.route),
        cms_collection = html_escape(pro_package::PAGE.metadata.cms_collection),
        eyebrow = html_escape(pro_package::HERO.eyebrow.unwrap_or("")),
        hero_title = html_escape(pro_package::HERO.title),
        hero_description = html_escape(pro_package::HERO.description),
        query_label = html_escape(pro_package::QUERY_FIELD.label),
        query_input_type = html_escape(pro_package::QUERY_FIELD.input_type),
        query_name = html_escape(pro_package::QUERY_FIELD.name),
        business_query = html_escape(form.business_query.trim()),
        query_placeholder = html_escape(pro_package::QUERY_FIELD.placeholder),
        category_label = html_escape(pro_package::CATEGORY_FIELD.label),
        category_name = html_escape(pro_package::CATEGORY_FIELD.name),
        category_options = render_category_options(form.business_category.trim()),
        submit_label = html_escape(pro_package::SUBMIT_LABEL),
        status_block = status_block,
        benefits = benefits,
        back_href = html_escape(pro_package::FOOTER.links[0].href),
        back_label = html_escape(pro_package::FOOTER.links[0].label),
        footer_note = html_escape(pro_package::FOOTER.note),
    )
}

pub fn render_pro_monitoring_page(
    scenario: &ProMonitoringScenario,
    events: &[ProMonitoringEvent],
) -> String {
    let latest_event = events.first();
    let example_summary = latest_event
        .map(|event| {
            format!(
                "{}: {}. Рекомендация: {}",
                event.competitor_name, event.change_summary, event.recommendation
            )
        })
        .unwrap_or_else(|| {
            "Система готова показать первое изменение после ежедневной проверки.".to_string()
        });

    let event_cards = if events.is_empty() {
        "<article class=\"log-card\"><strong>Изменений пока нет</strong><p>Как только мониторинг заметит обновления по конкурентам, они появятся здесь и в журнале уведомлений.</p></article>".to_string()
    } else {
        let mut items = String::new();
        for event in events.iter().take(3) {
            items.push_str(&format!(
                r#"<article class="log-card">
                  <strong>{competitor_name}</strong>
                  <p>{change_summary}</p>
                  <span>{changed_at_label}</span>
                </article>"#,
                competitor_name = html_escape(&event.competitor_name),
                change_summary = html_escape(&event.change_summary),
                changed_at_label = html_escape(&event.changed_at_label),
            ));
        }
        items
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 7. Действие PRO</title>
  <style>
    :root {{
      --bg: #f2efe7;
      --ink: #17231f;
      --muted: #5b6d66;
      --line: #d8ddd2;
      --card: rgba(255,255,255,0.95);
      --accent: #1d5947;
      --accent-soft: #e2efe8;
      --warm: #a86a1b;
      --warm-soft: #f6ead7;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #e7f0e6, transparent 30%),
        linear-gradient(180deg, #fbf8f2 0%, var(--bg) 100%);
    }}
    main {{
      width: min(1120px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 30px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, h3, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .hero, .grid, .list, .summary-box {{ display: grid; gap: 14px; }}
    .eyebrow {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      font-size: 0.8rem;
    }}
    .hero-top {{
      display: flex;
      justify-content: space-between;
      gap: 14px;
      flex-wrap: wrap;
      align-items: start;
    }}
    .status-badge {{
      display: inline-flex;
      align-items: center;
      min-height: 40px;
      padding: 0 14px;
      border-radius: 999px;
      background: var(--warm-soft);
      color: var(--warm);
      font-weight: 800;
    }}
    .grid {{
      grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    }}
    .metric, .summary-box, .log-card {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
    }}
    .metric strong {{
      display: block;
      font-size: 1.8rem;
      margin-bottom: 6px;
    }}
    .summary-box {{
      background: linear-gradient(180deg, rgba(255,255,255,0.97), rgba(226,239,232,0.92));
    }}
    .list {{
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }}
    .log-card strong {{
      display: block;
      margin-bottom: 8px;
    }}
    .log-card span {{
      color: var(--accent);
      font-size: 0.92rem;
      font-weight: 700;
    }}
    .cta-row {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-top: 4px;
    }}
    .button, .ghost {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      text-decoration: none;
      font-weight: 700;
    }}
    .button {{
      background: var(--accent);
      color: #fff;
    }}
    .ghost {{
      color: var(--accent);
      border: 1px solid var(--line);
      background: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <div class="hero-top">
        <div class="hero">
          <span class="eyebrow">Модуль 7. Действие PRO</span>
          <h1>Ежедневный автоматический мониторинг конкурентов</h1>
          <p>Модуль продолжает ветку оплаты и PRO-пакета: после проверки из `6.1` система ежедневно отслеживает рейтинг, новые отзывы, фото, новости и смену позиций по конкурентам.</p>
        </div>
        <div class="status-badge">Статус: {monitoring_status}</div>
      </div>

      <div class="grid">
        <article class="metric">
          <strong>1 раз в сутки</strong>
          <p>Частота проверки: {check_frequency}</p>
        </article>
        <article class="metric">
          <strong>5 сигналов</strong>
          <p>Рейтинг, отзывы, фото, новости и смена позиций.</p>
        </article>
        <article class="metric">
          <strong>{event_count}</strong>
          <p>Изменений уже зафиксировано в системном журнале.</p>
        </article>
        <article class="metric">
          <strong>PDF</strong>
          <p>Формат сводного отчёта по тону отзывов и изменениям.</p>
        </article>
      </div>
    </section>

    <section class="summary-box">
      <h2>Что мониторим по сценарию</h2>
      <p><strong>Точка входа:</strong> {business_query}</p>
      <p><strong>Категория бизнеса:</strong> {business_category}</p>
      <p><strong>Создано:</strong> {created_at_label}</p>
      <p><strong>Последний сигнал:</strong> {example_summary}</p>
    </section>

    <section>
      <h2>Пользователь видит уведомление в кабинете, например</h2>
      <div class="list" style="margin-top: 16px;">{event_cards}</div>
      <div class="cta-row">
        <a class="button" href="/pricing/pro/monitor/{scenario_id}/notifications">Открыть модуль 7.1</a>
        <a class="ghost" href="/pricing/pro/monitor/{scenario_id}/module-8">Перейти к модулю 8</a>
        <a class="ghost" href="/pricing/pro/monitor/{scenario_id}/module-9">Перейти к модулю 9</a>
        <a class="ghost" href="/pricing/pro">Создать новый PRO-сценарий</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        monitoring_status = html_escape(monitoring_status_label(&scenario.monitoring_status)),
        check_frequency = html_escape(monitoring_frequency_label(&scenario.check_frequency)),
        event_count = events.len(),
        business_query = html_escape(&scenario.business_query),
        business_category = html_escape(&scenario.business_category),
        created_at_label = html_escape(&scenario.created_at_label),
        example_summary = html_escape(&example_summary),
        event_cards = event_cards,
        scenario_id = scenario.id,
    )
}

pub fn render_pro_monitoring_notifications_page(
    scenario: &ProMonitoringScenario,
    events: &[ProMonitoringEvent],
) -> String {
    let journal_items = if events.is_empty() {
        "<article class=\"journal-item\"><h3>Изменений пока нет</h3><p>Когда ежедневная проверка найдёт изменения у конкурентов, они появятся в этом системном журнале.</p></article>".to_string()
    } else {
        let mut items = String::new();
        for event in events {
            items.push_str(&format!(
                r#"<article class="journal-item">
                  <div class="journal-top">
                    <div>
                      <h3>{competitor_name}</h3>
                      <p class="meta">{changed_at_label}</p>
                    </div>
                    <span class="field-pill">{changed_field}</span>
                  </div>
                  <p><strong>Что изменилось:</strong> {change_summary}</p>
                  <p><strong>Рекомендация:</strong> {recommendation}</p>
                </article>"#,
                competitor_name = html_escape(&event.competitor_name),
                changed_at_label = html_escape(&event.changed_at_label),
                changed_field = html_escape(&event.changed_field),
                change_summary = html_escape(&event.change_summary),
                recommendation = html_escape(&event.recommendation),
            ));
        }
        items
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 7.1. Уведомление в личном кабинете</title>
  <style>
    :root {{
      --bg: #eef1eb;
      --ink: #17231f;
      --muted: #607069;
      --line: #d6ddd2;
      --card: rgba(255,255,255,0.96);
      --accent: #1b5b48;
      --accent-soft: #e1efe8;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #ddeae0, transparent 26%),
        linear-gradient(180deg, #f9fbf8 0%, var(--bg) 100%);
    }}
    main {{
      width: min(980px, 100%);
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 30px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, h3, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .hero, .journal {{ display: grid; gap: 14px; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      font-size: 0.8rem;
    }}
    .summary {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: linear-gradient(180deg, rgba(255,255,255,0.98), rgba(225,239,232,0.9));
      display: grid;
      gap: 8px;
    }}
    .journal-item {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
      display: grid;
      gap: 10px;
    }}
    .journal-top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      flex-wrap: wrap;
      align-items: start;
    }}
    .meta {{
      font-size: 0.92rem;
    }}
    .field-pill {{
      display: inline-flex;
      min-height: 34px;
      align-items: center;
      padding: 0 12px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-size: 0.9rem;
      font-weight: 700;
    }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }}
    .link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 700;
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 7.1. Уведомление в личном кабинете</span>
      <h1>Системный журнал изменений по конкурентам</h1>
      <p>Экран продолжает модуль 7 и показывает, что именно сохраняется в кабинете: дата изменения, конкурент, что изменилось и какая рекомендация сформирована системой.</p>

      <div class="summary">
        <p><strong>Сценарий мониторинга:</strong> {business_query}</p>
        <p><strong>Категория:</strong> {business_category}</p>
        <p><strong>Формат общего отчёта:</strong> {report_format}</p>
      </div>
    </section>

    <section class="journal">
      <h2>Журнал уведомлений</h2>
      {journal_items}
      <div class="actions">
        <a class="link" href="/pricing/pro/monitor/{scenario_id}">Вернуться к модулю 7</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-8">Открыть модуль 8</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-9">Открыть модуль 9</a>
        <a class="link" href="/pricing/pro">Создать новый сценарий</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        business_query = html_escape(&scenario.business_query),
        business_category = html_escape(&scenario.business_category),
        report_format = html_escape(report_format_label(&scenario.report_format)),
        journal_items = journal_items,
        scenario_id = scenario.id,
    )
}

pub fn render_module_eight_page(
    scenario: &ProMonitoringScenario,
    form: &ModuleEightForm,
    issues: &[String],
    audits: &[ModuleEightAuditSummary],
) -> String {
    let issue_block = if issues.is_empty() {
        String::new()
    } else {
        format!(
            r#"<section class="errors"><strong>Система нашла ошибки в запросе:</strong><ul>{}</ul></section>"#,
            issues
                .iter()
                .map(|issue| format!("<li>{}</li>", html_escape(issue)))
                .collect::<Vec<_>>()
                .join("")
        )
    };

    let audit_cards = if audits.is_empty() {
        "<article class=\"audit-card\"><strong>Запусков ещё нет</strong><p>После отправки ссылки сюда попадёт отчёт по наполненности карточки, тону отзывов и сравнению с 5 конкурентами.</p></article>".to_string()
    } else {
        audits
            .iter()
            .map(|audit| {
                let card_title = audit
                    .card_title
                    .as_deref()
                    .unwrap_or("Карточка ещё обрабатывается");
                let score = audit
                    .completeness_score
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "в работе".to_string());
                let status_line = audit
                    .status_message
                    .as_deref()
                    .unwrap_or("Ожидаем результат");
                let platform_label = platform_label_from_db(&audit.source_platform);

                format!(
                    r#"<article class="audit-card">
                      <div class="audit-top">
                        <strong>{card_title}</strong>
                        <span class="status">{request_status}</span>
                      </div>
                      <p>{status_line}</p>
                      <p><strong>Платформа:</strong> {platform_label}</p>
                      <p><strong>Наполненность:</strong> {score}</p>
                      <p><strong>Запуск:</strong> {requested_at_label}</p>
                      <a class="audit-link" href="/pricing/pro/monitor/{scenario_id}/module-8/{audit_id}">Открыть отчёт</a>
                    </article>"#,
                    card_title = html_escape(card_title),
                    request_status = html_escape(request_status_label(&audit.request_status)),
                    status_line = html_escape(status_line),
                    platform_label = html_escape(platform_label),
                    score = html_escape(&score),
                    requested_at_label = html_escape(&audit.requested_at_label),
                    scenario_id = scenario.id,
                    audit_id = audit.audit_id,
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 8</title>
  <style>
    :root {{
      --bg: #f4efe6;
      --ink: #17231f;
      --muted: #617067;
      --line: #d8ddd3;
      --card: rgba(255,255,255,0.96);
      --accent: #234f7d;
      --accent-soft: #e4edf7;
      --warn: #935f17;
      --warn-soft: #f7ead6;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #edf2f9, transparent 28%),
        linear-gradient(180deg, #fbf8f3 0%, var(--bg) 100%);
    }}
    main {{ width: min(1120px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      font-size: 0.8rem;
    }}
    .hero, .grid, .audit-list, form {{ display: grid; gap: 14px; }}
    .grid {{ grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); }}
    .metric, .audit-card {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
    }}
    .metric strong {{ display: block; font-size: 1.7rem; margin-bottom: 6px; }}
    label {{ display: grid; gap: 8px; font-weight: 700; }}
    input {{
      min-height: 52px;
      border-radius: 14px;
      border: 1px solid var(--line);
      padding: 0 16px;
      font: inherit;
    }}
    button, .link {{
      display: inline-flex;
      width: fit-content;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      border: none;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 700;
      cursor: pointer;
    }}
    .errors {{
      border: 1px solid #edd7bc;
      background: var(--warn-soft);
      color: #6d4510;
    }}
    ul {{ margin: 0; padding-left: 20px; }}
    .audit-top {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: start;
      flex-wrap: wrap;
    }}
    .status {{
      color: var(--accent);
      font-size: 0.92rem;
      font-weight: 700;
    }}
    .audit-link {{
      color: var(--accent);
      font-weight: 700;
      text-decoration: none;
    }}
    .actions {{ display: flex; gap: 12px; flex-wrap: wrap; }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 8</span>
      <h1>Система анализирует Яндекс.Карты или Google Maps</h1>
      <p>По ссылке система реально загружает страницу карточки, разбирает доступный HTML и structured data, затем собирает краткий отчёт: наполненность, тон отзывов, рекомендации AI и сравнение с 5 конкурентами в ближайшем радиусе.</p>

      <div class="grid">
        <article class="metric">
          <strong>5 конкурентов</strong>
          <p>Сравнение идёт по локальному радиусу сценария `{business_query}`.</p>
        </article>
        <article class="metric">
          <strong>Наполненность</strong>
          <p>Фото, меню, контакты, соцсети и особенности карточки.</p>
        </article>
        <article class="metric">
          <strong>Отзывы</strong>
          <p>Система считает общий тон негативных и позитивных отзывов.</p>
        </article>
        <article class="metric">
          <strong>Контроль ошибок</strong>
          <p>Перед анализом модуль проверяет ссылку и диагностирует слабые места запроса.</p>
        </article>
      </div>
    </section>

    {issue_block}

    <section>
      <h2>Запуск анализа по ссылке</h2>
      <form action="/pricing/pro/monitor/{scenario_id}/module-8" method="post">
        <label>
          Ссылка на карточку Яндекс.Карт или Google Maps
          <input type="url" name="source_url" value="{source_url}" placeholder="https://yandex.ru/maps/org/... или https://maps.google.com/..." required />
        </label>
        <p>Сценарий: {business_query} · Категория: {business_category}</p>
        <button type="submit">Запустить модуль 8</button>
      </form>
    </section>

    <section class="audit-list">
      <h2>Последние запуски</h2>
      {audit_cards}
      <div class="actions">
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/notifications">Вернуться к модулю 7.1</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-10">Открыть модуль 10</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-9">Открыть модуль 9</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        issue_block = issue_block,
        scenario_id = scenario.id,
        source_url = html_escape(form.source_url.trim()),
        business_query = html_escape(&scenario.business_query),
        business_category = html_escape(&scenario.business_category),
        audit_cards = audit_cards,
    )
}

pub fn render_module_eight_audit_page(
    scenario: &ProMonitoringScenario,
    audit: &ModuleEightAuditRecord,
) -> String {
    let progress_percent = audit.progress_percent.unwrap_or(5).clamp(0, 100);
    let neutral_share =
        100 - audit.positive_review_share.unwrap_or(0) - audit.negative_review_share.unwrap_or(0);
    let recommendations = audit
        .ai_recommendations
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("<li>{}</li>", html_escape(line)))
        .collect::<Vec<_>>()
        .join("");
    let diagnostics = audit
        .request_diagnostics
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("<li>{}</li>", html_escape(line)))
        .collect::<Vec<_>>()
        .join("");
    let website_row = audit
        .website_url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
        .map(|url| {
            format!(
                r#"<a class="link" href="{url}" target="_blank" rel="noopener noreferrer">Сайт бизнеса</a>"#,
                url = html_escape(url),
            )
        })
        .unwrap_or_else(|| "<span>Сайт бизнеса не найден</span>".to_string());
    let social_links_list = audit
        .social_links
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            format!(
                r#"<li><a href="{url}" target="_blank" rel="noopener noreferrer">{url}</a></li>"#,
                url = html_escape(line),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let social_links_list = if social_links_list.is_empty() {
        "<li>Соцсети не обнаружены</li>".to_string()
    } else {
        social_links_list
    };
    let delivery_label = if audit.has_delivery.unwrap_or(false) {
        "Да"
    } else {
        "Не обнаружена"
    };
    let competitor_rows = audit
        .competitor_snapshot
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '|');
            Some((
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ))
        })
        .map(|(name, score, gap, focus)| {
            format!(
                r#"<article class="competitor-card">
                  <strong>{name}</strong>
                  <p><strong>Индекс карточки:</strong> {score}%</p>
                  <p><strong>Разница:</strong> {gap}</p>
                  <p>{focus}</p>
                </article>"#,
                name = html_escape(&name),
                score = html_escape(&score),
                gap = html_escape(&gap),
                focus = html_escape(&focus),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let refresh = if audit.request_status == "completed" || audit.request_status == "failed" {
        String::new()
    } else {
        "<meta http-equiv=\"refresh\" content=\"2\">".to_string()
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  {refresh}
  <title>Модуль 8: отчёт</title>
  <style>
    :root {{
      --bg: #eef2f5;
      --ink: #16232b;
      --muted: #5c6d76;
      --line: #d6dde3;
      --card: rgba(255,255,255,0.97);
      --accent: #245d78;
      --accent-soft: #e2eef4;
      --positive: #1c7d4f;
      --negative: #a24132;
      --neutral: #b6c0c8;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #dfeaf1, transparent 24%),
        linear-gradient(180deg, #fafcfd 0%, var(--bg) 100%);
    }}
    main {{ width: min(1180px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      font-size: 0.8rem;
    }}
    .hero, .grid, .bars, .competitors {{
      display: grid;
      gap: 14px;
    }}
    .grid, .competitors {{
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }}
    .metric, .competitor-card, .list-box {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
    }}
    .metric strong {{ display: block; font-size: 1.8rem; margin-bottom: 6px; }}
    .progress {{
      width: 100%;
      height: 14px;
      border-radius: 999px;
      background: #e7edf1;
      overflow: hidden;
    }}
    .progress > span {{
      display: block;
      height: 100%;
      width: {progress_percent}%;
      background: linear-gradient(90deg, #6fa3c0 0%, var(--accent) 100%);
    }}
    .tone-bar {{
      display: grid;
      grid-template-columns: {positive_share}% {neutral_share}% {negative_share}%;
      min-height: 20px;
      overflow: hidden;
      border-radius: 999px;
      background: #eef2f5;
    }}
    .tone-positive {{ background: var(--positive); }}
    .tone-neutral {{ background: var(--neutral); }}
    .tone-negative {{ background: var(--negative); }}
    ul {{ margin: 0; padding-left: 20px; }}
    .actions {{ display: flex; gap: 12px; flex-wrap: wrap; }}
    .link {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      text-decoration: none;
      font-weight: 700;
      background: var(--accent);
      color: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 8</span>
      <h1>{card_title}</h1>
      <p>Сценарий: {business_query} · Категория: {business_category}</p>
      <p>Источник: {platform_label} · Запрос создан: {requested_at_label}</p>
      <p>Оригинальная ссылка: {source_url}</p>
      <p>Канонический URL: {canonical_url}</p>
      <div class="progress"><span></span></div>
      <p>{status_message}</p>
      <p>{stage_or_error}</p>
    </section>

    <section class="grid">
      <article class="metric">
        <strong>{completeness_score}%</strong>
        <p>Наполненность карточки</p>
      </article>
      <article class="metric">
        <strong>{photo_count}</strong>
        <p>Фото в карточке</p>
      </article>
      <article class="metric">
        <strong>{menu_items_count}</strong>
        <p>Товары / меню</p>
      </article>
      <article class="metric">
        <strong>{contact_count}</strong>
        <p>Контакты и CTA</p>
      </article>
      <article class="metric">
        <strong>{social_count}</strong>
        <p>Соцсети</p>
      </article>
      <article class="metric">
        <strong>{feature_count}</strong>
        <p>Особенности карточки</p>
      </article>
      <article class="metric">
        <strong>{delivery_label}</strong>
        <p>Доставка еды</p>
      </article>
    </section>

    <section class="grid">
      <article class="list-box">
        <h2>Сайт и соцсети</h2>
        {website_row}
        <ul>{social_links_list}</ul>
      </article>
    </section>

    <section class="bars">
      <h2>График: общий тон отзывов</h2>
      <div class="tone-bar">
        <span class="tone-positive"></span>
        <span class="tone-neutral"></span>
        <span class="tone-negative"></span>
      </div>
      <p>Позитивные: {positive_share}% · Нейтральные: {neutral_share}% · Негативные: {negative_share}%</p>
    </section>

    <section class="grid">
      <article class="list-box">
        <h2>Рекомендации AI</h2>
        <ul>{recommendations}</ul>
      </article>
      <article class="list-box">
        <h2>Система анализирует запрос на ошибки</h2>
        <ul>{diagnostics}</ul>
      </article>
    </section>

    <section>
      <h2>Сравнение карточки с 5 конкурентами в ближайшем радиусе</h2>
      <div class="competitors">{competitor_rows}</div>
      <div class="actions">
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-11">Новый анализ</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-10">Открыть модуль 10</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-9">Открыть модуль 9</a>
        <a class="link" href="/pricing/pro/monitor/{scenario_id}/notifications">Назад к модулю 7.1</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        refresh = refresh,
        progress_percent = progress_percent,
        positive_share = audit.positive_review_share.unwrap_or(0).clamp(0, 100),
        neutral_share = neutral_share.clamp(0, 100),
        negative_share = audit.negative_review_share.unwrap_or(0).clamp(0, 100),
        card_title = html_escape(
            audit
                .card_title
                .as_deref()
                .unwrap_or("Карточка анализируется"),
        ),
        business_query = html_escape(&scenario.business_query),
        business_category = html_escape(&scenario.business_category),
        platform_label = html_escape(platform_label_from_db(&audit.source_platform)),
        requested_at_label = html_escape(&audit.requested_at_label),
        source_url = html_escape(&audit.source_url),
        canonical_url = html_escape(&audit.canonical_url),
        status_message = html_escape(
            audit
                .status_message
                .as_deref()
                .unwrap_or("Готовим отчёт по карточке"),
        ),
        stage_or_error = html_escape(if audit.request_status == "failed" {
            audit
                .error_summary
                .as_deref()
                .unwrap_or("Фоновая обработка завершилась с ошибкой.")
        } else {
            audit
                .current_stage
                .as_deref()
                .map(module_eight_stage_label)
                .unwrap_or("Подготовка запуска")
        },),
        completeness_score = audit.completeness_score.unwrap_or(0),
        photo_count = audit.photo_count.unwrap_or(0),
        menu_items_count = audit.menu_items_count.unwrap_or(0),
        contact_count = audit.contact_count.unwrap_or(0),
        social_count = audit.social_count.unwrap_or(0),
        feature_count = audit.feature_count.unwrap_or(0),
        delivery_label = delivery_label,
        website_row = website_row,
        social_links_list = social_links_list,
        recommendations = recommendations,
        diagnostics = diagnostics,
        competitor_rows = competitor_rows,
        scenario_id = scenario.id,
    )
}

pub fn render_module_nine_page(
    scenario: &ProMonitoringScenario,
    reports: &[ModuleNineReportSummary],
) -> String {
    let report_cards = if reports.is_empty() {
        "<article class=\"report-card empty\"><strong>История пока пуста</strong><p>Сначала завершите хотя бы один запуск в модуле 8, после чего здесь автоматически соберётся расширенный отчёт и PDF для скачивания.</p></article>".to_string()
    } else {
        reports
            .iter()
            .map(|report| {
                let pdf_label = match report.pdf_status.as_str() {
                    "ready" => report
                        .pdf_generated_at_label
                        .as_deref()
                        .map(|value| format!("PDF собран: {value}"))
                        .unwrap_or_else(|| "PDF собран".to_string()),
                    "processing" => "PDF собирается".to_string(),
                    "failed" => "PDF требует пересборки".to_string(),
                    _ => "PDF в очереди".to_string(),
                };

                format!(
                    r#"<article class="report-card">
                      <div class="report-top">
                        <div>
                          <strong>{business_name}</strong>
                          <p>{created_at_label}</p>
                        </div>
                        <span class="status">{pdf_label}</span>
                      </div>
                      <p><strong>Источник карт:</strong> {source_platform}</p>
                      <p><strong>Рейтинг карточки:</strong> {card_rating_label}</p>
                      <p><strong>SEO-оценка:</strong> {seo_score}/100</p>
                      <div class="actions">
                        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-9/{report_id}">Открыть отчёт</a>
                        <a class="button" href="/pricing/pro/monitor/{scenario_id}/module-9/{report_id}/download">Скачать PDF</a>
                      </div>
                    </article>"#,
                    business_name = html_escape(&report.business_name),
                    created_at_label = html_escape(&report.created_at_label),
                    pdf_label = html_escape(&pdf_label),
                    source_platform = html_escape(platform_label_from_db(&report.source_platform)),
                    card_rating_label = html_escape(&report.card_rating_label),
                    seo_score = report.seo_score,
                    scenario_id = scenario.id,
                    report_id = report.report_id,
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 9. Расширенная история запросов</title>
  <style>
    :root {{
      --bg: #f2eee8;
      --ink: #181d27;
      --muted: #616979;
      --line: #d9ddd1;
      --card: rgba(255,255,255,0.96);
      --accent: #274d86;
      --accent-soft: #e5edf9;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #edf2fb, transparent 24%),
        linear-gradient(180deg, #fbfaf7 0%, var(--bg) 100%);
    }}
    main {{ width: min(1080px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      font-size: 0.8rem;
    }}
    .hero, .history {{ display: grid; gap: 14px; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 14px;
    }}
    .metric, .report-card {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
      display: grid;
      gap: 10px;
    }}
    .report-top, .actions {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: start;
      flex-wrap: wrap;
    }}
    .status {{
      color: var(--accent);
      font-size: 0.92rem;
      font-weight: 700;
    }}
    .button {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 700;
    }}
    .button.secondary {{
      background: #fff;
      color: var(--accent);
      border: 1px solid var(--line);
    }}
    .empty strong {{ font-size: 1.05rem; }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 9. Расширенная история запросов</span>
      <h1>История готовых отчётов по завершённым анализам</h1>
      <p>Здесь сохраняются расширенные отчёты по каждому завершённому запуску из модуля 8: дата анализа, объект, источник карт, рейтинг карточки, SEO-разбор, конкуренты, рекомендации и PDF для скачивания.</p>

      <div class="grid">
        <article class="metric">
          <strong>{report_count}</strong>
          <p>готовых записей в истории сценария.</p>
        </article>
        <article class="metric">
          <strong>PDF storage</strong>
          <p>Каждый отчёт сохраняет PDF на диске и метаданные в базе.</p>
        </article>
        <article class="metric">
          <strong>{business_query}</strong>
          <p>активный кластер мониторинга для этого сценария.</p>
        </article>
      </div>
    </section>

    <section class="history">
      <h2>История отчётов</h2>
      {report_cards}
      <div class="actions">
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-10">Открыть модуль 10</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-11">Новый анализ</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/notifications">Вернуться к модулю 7.1</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        report_count = reports.len(),
        business_query = html_escape(&scenario.business_query),
        report_cards = report_cards,
        scenario_id = scenario.id,
    )
}

pub fn render_module_nine_report_page(
    scenario: &ProMonitoringScenario,
    report: &ModuleNineReportRecord,
) -> String {
    let competitor_items = report
        .competitors_summary
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("<li>{}</li>", html_escape(line)))
        .collect::<Vec<_>>()
        .join("");
    let recommendation_items = report
        .recommendations
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("<li>{}</li>", html_escape(line)))
        .collect::<Vec<_>>()
        .join("");
    let pdf_note = match report.pdf_status.as_str() {
        "ready" => report
            .pdf_generated_at_label
            .as_deref()
            .map(|value| format!("PDF сохранён: {value}"))
            .unwrap_or_else(|| "PDF сохранён и готов к скачиванию".to_string()),
        "processing" => "PDF ещё собирается".to_string(),
        "failed" => "PDF не удалось собрать автоматически".to_string(),
        _ => "PDF поставлен в очередь на сборку".to_string(),
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 9: отчёт</title>
  <style>
    :root {{
      --bg: #f1efe9;
      --ink: #18202a;
      --muted: #5f6876;
      --line: #d7ddcf;
      --card: rgba(255,255,255,0.96);
      --accent: #294c88;
      --accent-soft: #e8eef9;
      --negative: #a14a39;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top center, #e9eff8, transparent 24%),
        linear-gradient(180deg, #fbfaf7 0%, var(--bg) 100%);
    }}
    main {{ width: min(980px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      font-size: 0.8rem;
    }}
    .hero, .content {{
      display: grid;
      gap: 14px;
    }}
    .meta-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 14px;
    }}
    .meta, .box {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
    }}
    .meta strong {{
      display: block;
      font-size: 1.4rem;
      margin-bottom: 6px;
    }}
    .danger strong {{ color: var(--negative); }}
    ul {{ margin: 0; padding-left: 20px; }}
    .actions {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }}
    .button {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      text-decoration: none;
      font-weight: 700;
      background: var(--accent);
      color: #fff;
    }}
    .button.secondary {{
      background: #fff;
      color: var(--accent);
      border: 1px solid var(--line);
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 9</span>
      <h1>{business_name}</h1>
      <p>Расширенный отчёт по сценарию `{business_query}`. Внутри собраны дата анализа, источник карт, рейтинг карточки, SEO-анализ, конкуренты, рекомендации и сильные/слабые стороны.</p>
      <p><strong>{pdf_note}</strong></p>

      <div class="meta-grid">
        <article class="meta">
          <strong>{created_at_label}</strong>
          <p>дата анализа</p>
        </article>
        <article class="meta">
          <strong>{source_platform}</strong>
          <p>источник карт</p>
        </article>
        <article class="meta">
          <strong>{card_rating_label}</strong>
          <p>рейтинг карточки</p>
        </article>
        <article class="meta">
          <strong>{seo_score}/100</strong>
          <p>оценка SEO-оптимизации</p>
        </article>
      </div>
    </section>

    <section class="content">
      <article class="box">
        <h2>Конкуренты</h2>
        <ul>{competitor_items}</ul>
      </article>
      <article class="box">
        <h2>Рекомендации</h2>
        <ul>{recommendation_items}</ul>
      </article>
      <article class="box">
        <h2>Сильные стороны</h2>
        <p>{strengths}</p>
      </article>
      <article class="box danger">
        <h2>Слабые стороны</h2>
        <p>{weaknesses}</p>
      </article>
      <article class="box">
        <h2>Анализ SEO</h2>
        <p>{seo_analysis}</p>
      </article>
      <div class="actions">
        <a class="button" href="/pricing/pro/monitor/{scenario_id}/module-9/{report_id}/download">Скачать PDF</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-10">Фоновый мониторинг</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-11">Новый анализ</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-9">Назад в историю</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        business_name = html_escape(&report.business_name),
        business_query = html_escape(&scenario.business_query),
        pdf_note = html_escape(&pdf_note),
        created_at_label = html_escape(&report.created_at_label),
        source_platform = html_escape(platform_label_from_db(&report.source_platform)),
        card_rating_label = html_escape(&report.card_rating_label),
        seo_score = report.seo_score,
        competitor_items = competitor_items,
        recommendation_items = recommendation_items,
        strengths = html_escape(&report.strengths),
        weaknesses = html_escape(&report.weaknesses),
        seo_analysis = html_escape(&report.seo_analysis),
        scenario_id = scenario.id,
        report_id = report.report_id,
    )
}

pub fn render_category_options(selected: &str) -> String {
    let mut options = String::new();
    // AI assistants: category copy is managed in `content/pages/pro_package.rs`.
    for option in pro_package::CATEGORY_FIELD.options {
        let selected_attr = if option.value == selected {
            " selected"
        } else {
            ""
        };
        let value = html_escape(option.value);
        options.push_str(&format!(
            r#"<option value="{value}"{selected_attr}>{label}</option>"#,
            value = value,
            selected_attr = selected_attr,
            label = html_escape(option.label),
        ));
    }

    options
}

pub fn render_module_ten_page(
    scenario: &ProMonitoringScenario,
    events: &[ProMonitoringEvent],
    audits: &[ModuleEightAuditSummary],
    reports: &[ModuleNineReportSummary],
) -> String {
    let last_event = events
        .first()
        .map(|event| {
            format!(
                "{}: {}",
                event.changed_at_label,
                report_excerpt(Some(&event.change_summary))
            )
        })
        .unwrap_or_else(|| "Изменения ещё не зафиксированы".to_string());
    let last_audit = audits
        .first()
        .and_then(|audit| audit.card_title.as_deref())
        .unwrap_or("Запусков анализа пока нет");
    let latest_report = reports
        .first()
        .map(|report| report.created_at_label.as_str())
        .unwrap_or("ещё не сформирован");

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 10. Фоновый мониторинг</title>
  <style>
    :root {{
      --bg: #eef3ed;
      --ink: #18231d;
      --muted: #5e6d64;
      --line: #d5ddd5;
      --card: rgba(255,255,255,0.97);
      --accent: #1f5b4b;
      --accent-soft: #e2f1eb;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top right, #e7f0ea, transparent 24%),
        linear-gradient(180deg, #f9fcf9 0%, var(--bg) 100%);
    }}
    main {{ width: min(1080px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      font-size: 0.8rem;
    }}
    .hero, .grid, .actions {{ display: grid; gap: 14px; }}
    .grid {{ grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); }}
    .metric {{
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      background: #fff;
    }}
    .metric strong {{ display: block; font-size: 1.6rem; margin-bottom: 6px; }}
    .button {{
      display: inline-flex;
      width: fit-content;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 700;
    }}
    .button.secondary {{
      background: #fff;
      color: var(--accent);
      border: 1px solid var(--line);
    }}
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="pill">Модуль 10. Фоновый мониторинг</span>
      <h1>Сценарий работает в фоне и собирает сигналы по карточке бизнеса</h1>
      <p>Модуль 10 логично продолжает модуль 9: после просмотра истории отчётов здесь видно состояние активного мониторинга, последние изменения по конкурентам и точка входа в новый анализ карточки.</p>

      <div class="grid">
        <article class="metric">
          <strong>{monitoring_status}</strong>
          <p>статус фонового сценария для кластера `{business_query}`.</p>
        </article>
        <article class="metric">
          <strong>{event_count}</strong>
          <p>событий уже записано в журнал изменений.</p>
        </article>
        <article class="metric">
          <strong>{audit_count}</strong>
          <p>ручных запусков анализа карточки уже собрано.</p>
        </article>
        <article class="metric">
          <strong>{report_count}</strong>
          <p>расширенных отчётов и PDF сформировано системой.</p>
        </article>
      </div>
    </section>

    <section class="hero">
      <h2>Что происходит сейчас</h2>
      <p><strong>Категория бизнеса:</strong> {business_category}</p>
      <p><strong>Последнее событие:</strong> {last_event}</p>
      <p><strong>Последний анализ:</strong> {last_audit}</p>
      <p><strong>Последний расширенный отчёт:</strong> {latest_report}</p>
      <div class="actions">
        <a class="button" href="/pricing/pro/monitor/{scenario_id}/module-11">Новый анализ</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/module-9">Вернуться к модулю 9</a>
        <a class="button secondary" href="/pricing/pro/monitor/{scenario_id}/notifications">Журнал уведомлений</a>
      </div>
    </section>
  </main>
</body>
</html>"#,
        monitoring_status = html_escape(monitoring_status_label(&scenario.monitoring_status)),
        business_query = html_escape(&scenario.business_query),
        event_count = events.len(),
        audit_count = audits.len(),
        report_count = reports.len(),
        business_category = html_escape(&scenario.business_category),
        last_event = html_escape(&last_event),
        last_audit = html_escape(last_audit),
        latest_report = html_escape(latest_report),
        scenario_id = scenario.id,
    )
}

pub fn render_module_eleven_page(scenario: &ProMonitoringScenario) -> String {
    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 11. Новый анализ</title>
  <style>
    :root {{
      --bg: #f3efe8;
      --ink: #1d1f27;
      --muted: #666d78;
      --line: #d9dcd3;
      --card: rgba(255,255,255,0.97);
      --accent: #2f476f;
      --accent-soft: #e6ecf7;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at center, #e9eef7, transparent 28%),
        linear-gradient(180deg, #fbfaf7 0%, var(--bg) 100%);
    }}
    main {{
      width: min(760px, 100%);
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 28px;
      padding: 40px 32px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
      display: grid;
      gap: 18px;
      text-align: center;
    }}
    h1, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .button {{
      display: inline-flex;
      justify-content: center;
      align-items: center;
      min-height: 58px;
      padding: 0 28px;
      border-radius: 16px;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 800;
      font-size: 1.02rem;
    }}
    .link {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 700;
    }}
  </style>
</head>
<body>
  <main>
    <h1>Модуль 11. Новый анализ</h1>
    <p>Для сценария `{business_query}` можно запустить новый анализ карточки. Кнопка ниже переводит на отдельную страницу ввода, где система проверит ссылку, категорию бизнеса и сразу отправит запуск в существующий движок анализа.</p>
    <a class="button" href="/pricing/pro/monitor/{scenario_id}/module-12">Новый анализ карточки</a>
    <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-10">Назад к модулю 10</a>
  </main>
</body>
</html>"#,
        business_query = html_escape(&scenario.business_query),
        scenario_id = scenario.id,
    )
}

pub fn render_module_twelve_page(
    scenario: &ProMonitoringScenario,
    form: &ModuleTwelveForm,
    issues: &[String],
) -> String {
    let issue_block = if issues.is_empty() {
        String::new()
    } else {
        format!(
            r#"<section class="errors"><strong>Система анализирует запрос на ошибки:</strong><ul>{}</ul></section>"#,
            issues
                .iter()
                .map(|issue| format!("<li>{}</li>", html_escape(issue)))
                .collect::<Vec<_>>()
                .join("")
        )
    };

    format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Модуль 12. Страница нового анализа</title>
  <style>
    :root {{
      --bg: #f4efe6;
      --ink: #17231f;
      --muted: #617067;
      --line: #d8ddd3;
      --card: rgba(255,255,255,0.96);
      --accent: #234f7d;
      --accent-soft: #e4edf7;
      --warn: #935f17;
      --warn-soft: #f7ead6;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      padding: 24px;
      font-family: "Segoe UI", "Helvetica Neue", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, #edf2f9, transparent 28%),
        linear-gradient(180deg, #fbf8f3 0%, var(--bg) 100%);
    }}
    main {{ width: min(920px, 100%); margin: 0 auto; display: grid; gap: 18px; }}
    section {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 28px;
      box-shadow: 0 18px 45px rgba(22, 35, 30, 0.08);
    }}
    h1, h2, p {{ margin: 0; }}
    p {{ color: var(--muted); line-height: 1.65; }}
    .pill {{
      display: inline-flex;
      width: fit-content;
      padding: 8px 14px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      font-size: 0.8rem;
    }}
    form {{ display: grid; gap: 16px; }}
    label {{ display: grid; gap: 8px; font-weight: 700; }}
    input, select {{
      min-height: 52px;
      border-radius: 14px;
      border: 1px solid var(--line);
      padding: 0 16px;
      font: inherit;
      background: #fff;
    }}
    button, .link {{
      display: inline-flex;
      width: fit-content;
      align-items: center;
      justify-content: center;
      min-height: 48px;
      padding: 0 18px;
      border-radius: 12px;
      border: none;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      font-weight: 700;
      cursor: pointer;
    }}
    .errors {{
      border: 1px solid #edd7bc;
      background: var(--warn-soft);
      color: #6d4510;
    }}
    ul {{ margin: 0; padding-left: 20px; }}
    .actions {{ display: flex; gap: 12px; flex-wrap: wrap; }}
  </style>
</head>
<body>
  <main>
    <section>
      <span class="pill">Модуль 12. Страница нового анализа</span>
      <h1>Запуск нового анализа карточки</h1>
      <p>Введите адрес или ссылку на карточку, либо вставьте точную ссылку на Яндекс.Карты / Google Maps. Система сама определит источник, проверит запрос на ошибки и запустит существующий движок анализа из модуля 8.</p>
      <p>Текущий сценарий: {business_query}</p>
    </section>

    {issue_block}

    <section>
      <form action="/pricing/pro/monitor/{scenario_id}/module-12" method="post">
        <label>
          Введите адрес / вставьте ссылку на карточку
          <input type="url" name="source_url" value="{source_url}" placeholder="https://yandex.ru/maps/org/... или https://maps.google.com/..." required />
        </label>
        <label>
          Выберите категорию бизнеса
          <select name="business_category" required>
            {category_options}
          </select>
        </label>
        <div class="actions">
          <button type="submit">Анализировать</button>
          <a class="link" href="/pricing/pro/monitor/{scenario_id}/module-10">Назад к модулю 10</a>
        </div>
      </form>
    </section>
  </main>
</body>
</html>"#,
        business_query = html_escape(&scenario.business_query),
        issue_block = issue_block,
        scenario_id = scenario.id,
        source_url = html_escape(form.source_url.trim()),
        category_options = render_category_options(&form.business_category),
    )
}
