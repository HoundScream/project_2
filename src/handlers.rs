use crate::analysis::{run_analysis_engine, run_module_eight_audit};
use crate::db::{
    apply_subscription_action, create_pro_monitoring_scenario, create_user_session,
    fetch_analysis_status_record, fetch_auth_user, fetch_module_eight_audit,
    fetch_module_eight_audits, fetch_module_nine_report, fetch_module_nine_reports,
    fetch_pro_monitoring_events, fetch_pro_monitoring_scenario, fetch_registered_user,
    fetch_session_user, fetch_user_analysis_requests, has_duplicate_business_profile,
    sync_module_nine_reports,
};
use crate::models::AppState;
use crate::pages::{
    render_analysis_status_page, render_cabinet_page, render_landing_page, render_login_page,
    render_module_eight_audit_page, render_module_eight_page, render_module_eleven_page,
    render_module_nine_page, render_module_nine_report_page, render_module_ten_page,
    render_module_twelve_page, render_not_found_page, render_pricing_page,
    render_pro_monitoring_notifications_page, render_pro_monitoring_page, render_pro_package_page,
    render_register_page, render_register_success_page, render_subscription_page,
};
use crate::utils::{
    analyze_query, build_session_cookie, collapse_whitespace, expire_session_cookie, hash_password,
    html_escape, normalize_phone, normalize_telegram_handle, parse_business_profile_url,
    redirect_with_cookie, resolve_business_profile_url, validate_pro_package_form,
    validate_registration_form, validate_subscription_access, validation_issue, verify_password,
};
use axum::{
    Form,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use bytes::Bytes;
use geonetix_core::{
    CabinetAnalysisForm, GeoObject, LoginForm, MapPlatform, ModuleEightForm, ModuleTwelveForm,
    ParsedBusinessUrl, ProPackageForm, RegisterForm, RegisteredUser, SearchParams,
    SubscriptionActionForm, SubscriptionFeedback, ValidationIssue,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn root() -> Html<String> {
    Html(render_landing_page())
}

pub async fn pricing_page() -> Html<String> {
    Html(render_pricing_page())
}

pub async fn pro_package_page() -> Html<String> {
    Html(render_pro_package_page(
        &ProPackageForm {
            business_query: String::new(),
            business_category: String::new(),
        },
        &[],
    ))
}

pub async fn pro_package_submit(
    State(state): State<AppState>,
    Form(form): Form<ProPackageForm>,
) -> Response {
    let issues = validate_pro_package_form(&form);
    if !issues.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(render_pro_package_page(&form, &issues)),
        )
            .into_response();
    }

    let scenario_id = match create_pro_monitoring_scenario(&state.pool, &form).await {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_pro_package_page(
                    &form,
                    &[
                        "Не удалось подготовить PRO-мониторинг. Попробуйте ещё раз немного позже."
                            .to_string(),
                    ],
                )),
            )
                .into_response();
        }
    };

    Redirect::to(&format!("/pricing/pro/monitor/{scenario_id}")).into_response()
}

pub async fn register_page() -> Html<String> {
    Html(render_register_page(
        &RegisterForm {
            full_name: String::new(),
            phone: String::new(),
            telegram_handle: String::new(),
            email: String::new(),
            password: String::new(),
        },
        &[],
    ))
}

pub async fn login_page() -> Html<String> {
    Html(render_login_page(
        &LoginForm {
            identifier: String::new(),
            password: String::new(),
        },
        &[],
    ))
}

pub async fn register_user(
    State(state): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> Response {
    let errors = validate_registration_form(&form);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(render_register_page(&form, &errors)),
        )
            .into_response();
    }

    let normalized_phone = normalize_phone(&form.phone);
    let normalized_telegram = normalize_telegram_handle(&form.telegram_handle);
    let normalized_email = form.email.trim().to_lowercase();
    let normalized_name = collapse_whitespace(&form.full_name);
    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_register_page(
                    &form,
                    &["Не удалось безопасно подготовить пароль. Попробуйте ещё раз.".to_string()],
                )),
            )
                .into_response();
        }
    };

    let created_user = match sqlx::query_as::<_, RegisteredUser>(
        r#"
        insert into app_users (
            full_name,
            phone,
            telegram_handle,
            email,
            password_hash
        )
        values ($1, $2, $3, $4, $5)
        returning
            id,
            full_name,
            phone,
            telegram_handle,
            email,
            account_tier,
            subscription_plan,
            to_char(subscription_expires_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as subscription_expires_at_label,
            subscription_auto_renew,
            subscription_cancel_at_period_end,
            case
                when account_tier = 'pro'
                    and (subscription_expires_at is null or subscription_expires_at >= now())
                then true
                else false
            end as subscription_is_active,
            analysis_quota_limit,
            analysis_quota_used
        "#,
    )
    .bind(normalized_name)
    .bind(normalized_phone)
    .bind(normalized_telegram)
    .bind(normalized_email)
    .bind(password_hash)
    .fetch_one(&state.pool)
    .await
    {
        Ok(user) => user,
        Err(error) => {
            if let Some(db_error) = error.as_database_error()
                && db_error.code().as_deref() == Some("23505") {
                    let message = db_error.constraint().map_or_else(
                        || "Пользователь с такими данными уже зарегистрирован.".to_string(),
                        |constraint| match constraint {
                            "app_users_email_key" => {
                                "Этот email уже используется в системе.".to_string()
                            }
                            "app_users_phone_key" => {
                                "Этот телефон уже используется в системе.".to_string()
                            }
                            "app_users_telegram_handle_key" => {
                                "Этот Telegram-ник уже используется в системе.".to_string()
                            }
                            _ => "Пользователь с такими данными уже зарегистрирован.".to_string(),
                        },
                    );

                    return (
                        StatusCode::CONFLICT,
                        Html(render_register_page(&form, &[message])),
                    )
                        .into_response();
                }

            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_register_page(
                    &form,
                    &[
                        "Не удалось сохранить пользователя. Попробуйте ещё раз чуть позже."
                            .to_string(),
                    ],
                )),
            )
                .into_response();
        }
    };

    let session_token = match create_user_session(&state.pool, created_user.id).await {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_register_page(
                    &form,
                    &["Пользователь создан, но не удалось открыть сессию. Попробуйте войти через форму логина.".to_string()],
                )),
            )
                .into_response();
        }
    };

    redirect_with_cookie(
        &format!("/register/success/{}", created_user.id),
        &build_session_cookie(&session_token),
    )
}

pub async fn login_user(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let identifier = collapse_whitespace(&form.identifier);
    if identifier.is_empty() || form.password.trim().is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(render_login_page(
                &form,
                &["Укажите email или телефон и пароль.".to_string()],
            )),
        )
            .into_response();
    }

    let normalized_email = identifier.to_lowercase();
    let normalized_phone = normalize_phone(&identifier);
    let auth_user = match fetch_auth_user(&state.pool, &normalized_email, &normalized_phone).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Html(render_login_page(
                    &form,
                    &["Пользователь не найден или пароль не совпадает.".to_string()],
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_login_page(
                    &form,
                    &["Не удалось выполнить вход. Попробуйте ещё раз чуть позже.".to_string()],
                )),
            )
                .into_response();
        }
    };

    let Some(password_hash) = auth_user.password_hash.as_deref() else {
        return (
            StatusCode::UNAUTHORIZED,
            Html(render_login_page(
                &form,
                &["Для этого аккаунта пароль ещё не настроен. Создайте новый тестовый аккаунт через регистрацию.".to_string()],
            )),
        )
            .into_response();
    };

    if !verify_password(password_hash, &form.password) {
        return (
            StatusCode::UNAUTHORIZED,
            Html(render_login_page(
                &form,
                &["Пользователь не найден или пароль не совпадает.".to_string()],
            )),
        )
            .into_response();
    }

    let session_token = match create_user_session(&state.pool, auth_user.id).await {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_login_page(
                    &form,
                    &[
                        "Не удалось создать сессию входа. Попробуйте повторить вход позже."
                            .to_string(),
                    ],
                )),
            )
                .into_response();
        }
    };

    redirect_with_cookie(
        &format!("/cabinet/{}", auth_user.id),
        &build_session_cookie(&session_token),
    )
}

pub async fn logout_user() -> Response {
    redirect_with_cookie("/login", &expire_session_cookie())
}

pub async fn register_success(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Response {
    let session_user = match require_authenticated_user(&state.pool, &headers, user_id).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    let user = match sqlx::query_as::<_, RegisteredUser>(
        r#"
        select
            id,
            full_name,
            phone,
            telegram_handle,
            email,
            account_tier,
            subscription_plan,
            to_char(subscription_expires_at at time zone 'UTC', 'YYYY-MM-DD HH24:MI UTC') as subscription_expires_at_label,
            subscription_auto_renew,
            subscription_cancel_at_period_end,
            case
                when account_tier = 'pro'
                    and (subscription_expires_at is null or subscription_expires_at >= now())
                then true
                else false
            end as subscription_is_active,
            analysis_quota_limit,
            analysis_quota_used
        from app_users
        where id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Регистрация не найдена",
                    "Пользователь не найден. Возможно, ссылка устарела или запись была удалена.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Ошибка чтения",
                    "Не удалось загрузить данные регистрации. Попробуйте обновить страницу позже.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_register_success_page(&session_user, &user)),
    )
        .into_response()
}

pub async fn cabinet_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_authenticated_user(&state.pool, &headers, user_id).await {
        return response;
    }

    render_cabinet_response(
        state,
        user_id,
        CabinetAnalysisForm {
            source_url: String::new(),
        },
        &[],
        StatusCode::OK,
    )
    .await
}

pub async fn subscription_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_authenticated_user(&state.pool, &headers, user_id).await {
        return response;
    }

    render_subscription_response(state, user_id, None, StatusCode::OK).await
}

pub async fn subscription_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Form(form): Form<SubscriptionActionForm>,
) -> Response {
    if let Err(response) = require_authenticated_user(&state.pool, &headers, user_id).await {
        return response;
    }

    let message = match apply_subscription_action(&state.pool, user_id, &form.action).await {
        Ok(message) => Some(message),
        Err(error_message) => {
            return render_subscription_response(
                state,
                user_id,
                Some(SubscriptionFeedback {
                    title: "Не удалось обновить подписку".to_string(),
                    detail: error_message,
                    is_error: true,
                }),
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
    };

    render_subscription_response(state, user_id, message, StatusCode::OK).await
}

pub async fn create_analysis_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Form(form): Form<CabinetAnalysisForm>,
) -> Response {
    if let Err(response) = require_authenticated_user(&state.pool, &headers, user_id).await {
        return response;
    }

    let mut issues = Vec::new();

    let parsed_url = match parse_business_profile_url(&form.source_url) {
        Ok(parsed_url) => Some(parsed_url),
        Err(error) => {
            issues.push(validation_issue("Неверная ссылка", error, false));
            None
        }
    };

    let user = match fetch_registered_user(&state.pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Кабинет не найден",
                    "Пользователь не найден. Проверьте ссылку на кабинет или зарегистрируйтесь заново.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Ошибка чтения",
                    "Не удалось загрузить кабинет. Попробуйте обновить страницу позже.",
                )),
            )
                .into_response();
        }
    };

    if let Some(limit_issue) = validate_subscription_access(&user) {
        issues.push(limit_issue);
    }

    let parsed_url = if let Some(parsed_url) = parsed_url {
        match resolve_business_profile_url(parsed_url).await {
            Ok(parsed_url) => Some(parsed_url),
            Err(issue) => {
                issues.push(issue);
                None
            }
        }
    } else {
        None
    };

    if let Some(parsed_url) = parsed_url.as_ref() {
        issues.extend(validate_analysis_submission(&state.pool, user.id, parsed_url).await);
    }

    if !issues.is_empty() {
        let records = fetch_user_analysis_requests(&state.pool, user_id)
            .await
            .unwrap_or_default();

        return render_cabinet_page(
            StatusCode::UNPROCESSABLE_ENTITY,
            &user,
            &form,
            &issues,
            &records,
        )
        .into_response();
    }

    let parsed_url = parsed_url.expect("parsed url should exist after validation");
    let canonical_url = parsed_url.canonical_url.clone();
    let profile_result = sqlx::query_as::<_, (Uuid,)>(
        r#"
        insert into business_profiles (
            app_user_id,
            source_platform,
            source_url,
            canonical_url
        )
        values ($1, $2, $3, $4)
        on conflict (app_user_id, canonical_url)
        do update set
            source_url = excluded.source_url,
            updated_at = now()
        returning id
        "#,
    )
    .bind(user.id)
    .bind(parsed_url.source_platform.as_db_value())
    .bind(parsed_url.source_url)
    .bind(canonical_url.clone())
    .fetch_one(&state.pool)
    .await;

    let (business_profile_id,) = match profile_result {
        Ok(row) => row,
        Err(_) => {
            return render_cabinet_response(
                state,
                user_id,
                form,
                &[validation_issue(
                    "Попробуйте позже",
                    "Не удалось сохранить карточку бизнеса. Попробуйте ещё раз немного позже.",
                    false,
                )],
                StatusCode::SERVICE_UNAVAILABLE,
            )
            .await;
        }
    };

    let request_result = sqlx::query_as::<_, (Uuid,)>(
        r#"
        insert into analysis_requests (
            app_user_id,
            business_profile_id,
            request_status,
            current_stage,
            stage_index,
            total_stages,
            progress_percent,
            status_message,
            status_detail
        )
        values ($1, $2, 'queued', 'validate_link', 1, 6, 5, 'Запускаем анализ', 'Готовим запрос к обработке')
        returning id
        "#,
    )
    .bind(user.id)
    .bind(business_profile_id)
    .fetch_one(&state.pool)
    .await;

    let (request_id,) = match request_result {
        Ok(row) => row,
        Err(_) => {
            return render_cabinet_response(
                state,
                user_id,
                form,
                &[validation_issue(
                    "Попробуйте позже",
                    "Не удалось создать запрос на анализ. Попробуйте ещё раз немного позже.",
                    false,
                )],
                StatusCode::SERVICE_UNAVAILABLE,
            )
            .await;
        }
    };

    let usage_update = sqlx::query(
        r#"
        update app_users
        set analysis_quota_used = analysis_quota_used + 1,
            updated_at = now()
        where id = $1
        "#,
    )
    .bind(user.id)
    .execute(&state.pool)
    .await;

    if usage_update.is_err() {
        return render_cabinet_response(
            state,
            user_id,
            form,
            &[validation_issue(
                "Попробуйте позже",
                "Запрос создан, но не удалось обновить лимит. Проверьте кабинет чуть позже.",
                false,
            )],
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
    }

    tokio::spawn(run_analysis_engine(
        state.pool.clone(),
        state.redis.clone(),
        user.id,
        request_id,
        business_profile_id,
        parsed_url.source_platform,
        canonical_url,
    ));

    Redirect::to(&format!("/cabinet/{user_id}/analysis/{request_id}")).into_response()
}

pub async fn analysis_status_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((user_id, request_id)): Path<(Uuid, Uuid)>,
) -> Response {
    if let Err(response) = require_authenticated_user(&state.pool, &headers, user_id).await {
        return response;
    }

    let user = match fetch_registered_user(&state.pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Кабинет не найден",
                    "Пользователь не найден. Проверьте ссылку на кабинет или зарегистрируйтесь заново.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Ошибка чтения",
                    "Не удалось загрузить кабинет. Попробуйте обновить страницу позже.",
                )),
            )
                .into_response();
        }
    };

    let analysis = match fetch_analysis_status_record(&state.pool, user_id, request_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Анализ не найден",
                    "Запрос анализа не найден или не относится к этому кабинету.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Статус анализа недоступен",
                    "Не удалось загрузить карточку анализа. Попробуйте открыть страницу позже.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_analysis_status_page(&user, &analysis)),
    )
        .into_response()
}

pub async fn pro_monitoring_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "PRO-мониторинг не найден",
                    "Сценарий мониторинга не найден. Запустите его заново из модуля 6.1.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Мониторинг недоступен",
                    "Не удалось загрузить данные ежедневного мониторинга. Попробуйте позже.",
                )),
            )
                .into_response();
        }
    };

    let events = match fetch_pro_monitoring_events(&state.pool, scenario_id).await {
        Ok(rows) => rows,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Журнал недоступен",
                    "Не удалось загрузить журнал изменений для этого сценария мониторинга.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_pro_monitoring_page(&scenario, &events)),
    )
        .into_response()
}

pub async fn pro_monitoring_notifications_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Уведомления не найдены",
                    "Сценарий мониторинга не найден. Вернитесь к модулю 6.1 и создайте новый запуск.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Уведомления недоступны",
                    "Не удалось загрузить системный журнал изменений. Попробуйте позже.",
                )),
            )
                .into_response();
        }
    };

    let events = match fetch_pro_monitoring_events(&state.pool, scenario_id).await {
        Ok(rows) => rows,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Журнал недоступен",
                    "Не удалось открыть список уведомлений по мониторингу.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_pro_monitoring_notifications_page(&scenario, &events)),
    )
        .into_response()
}

pub async fn module_eight_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 8 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в модуле PRO.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 8 недоступен",
                    "Не удалось загрузить сценарий для углублённого анализа карточки.",
                )),
            )
                .into_response();
        }
    };

    let audits = fetch_module_eight_audits(&state.pool, scenario_id)
        .await
        .unwrap_or_default();

    (
        StatusCode::OK,
        Html(render_module_eight_page(
            &scenario,
            &ModuleEightForm {
                source_url: String::new(),
            },
            &[],
            &audits,
        )),
    )
        .into_response()
}

pub async fn module_eight_submit(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
    Form(form): Form<ModuleEightForm>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 8 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в модуле PRO.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 8 недоступен",
                    "Не удалось загрузить сценарий для запуска анализа карточки.",
                )),
            )
                .into_response();
        }
    };

    let mut issues = Vec::new();
    let parsed_url = match parse_business_profile_url(&form.source_url) {
        Ok(parsed_url) => {
            if matches!(parsed_url.source_platform, MapPlatform::TwoGis) {
                issues.push(
                    "Для модуля 8 сейчас поддерживаются только ссылки Яндекс.Карт и Google Maps."
                        .to_string(),
                );
                None
            } else {
                Some(parsed_url)
            }
        }
        Err(error) => {
            issues.push(error);
            None
        }
    };

    let parsed_url = if let Some(parsed_url) = parsed_url {
        match resolve_business_profile_url(parsed_url).await {
            Ok(parsed_url) => Some(parsed_url),
            Err(issue) => {
                issues.push(format!("{} {}", issue.title, issue.detail));
                None
            }
        }
    } else {
        None
    };

    if !issues.is_empty() {
        let audits = fetch_module_eight_audits(&state.pool, scenario_id)
            .await
            .unwrap_or_default();

        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(render_module_eight_page(&scenario, &form, &issues, &audits)),
        )
            .into_response();
    }

    let parsed_url = parsed_url.expect("module 8 url should be parsed after validation");

    let (audit_id,) = match sqlx::query_as::<_, (Uuid,)>(
        r#"
        insert into module_eight_audits (
            scenario_id,
            source_platform,
            source_url,
            canonical_url,
            request_status,
            current_stage,
            progress_percent,
            status_message
        )
        values ($1, $2, $3, $4, 'queued', 'validate_request', 5, 'Готовим запуск анализа')
        returning id
        "#,
    )
    .bind(scenario_id)
    .bind(parsed_url.source_platform.as_db_value())
    .bind(&parsed_url.source_url)
    .bind(&parsed_url.canonical_url)
    .fetch_one(&state.pool)
    .await
    {
        Ok(row) => row,
        Err(_) => {
            let audits = fetch_module_eight_audits(&state.pool, scenario_id)
                .await
                .unwrap_or_default();

            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_module_eight_page(
                    &scenario,
                    &form,
                    &[
                        "Не удалось сохранить запуск модуля 8. Попробуйте ещё раз чуть позже."
                            .to_string(),
                    ],
                    &audits,
                )),
            )
                .into_response();
        }
    };

    tokio::spawn(run_module_eight_audit(
        state.pool.clone(),
        state.redis.clone(),
        scenario.id,
        audit_id,
        parsed_url.source_platform,
        parsed_url.canonical_url,
    ));

    Redirect::to(&format!(
        "/pricing/pro/monitor/{scenario_id}/module-8/{audit_id}"
    ))
    .into_response()
}

pub async fn module_eight_audit_page(
    State(state): State<AppState>,
    Path((scenario_id, audit_id)): Path<(Uuid, Uuid)>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 8 не найден",
                    "Сценарий мониторинга не найден. Вернитесь к модулю 7.1 и запустите новый сценарий.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 8 недоступен",
                    "Не удалось загрузить сценарий мониторинга для детального анализа.",
                )),
            )
                .into_response();
        }
    };

    let audit = match fetch_module_eight_audit(&state.pool, scenario_id, audit_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Запуск модуля 8 не найден",
                    "Такой запуск не найден или не относится к выбранному сценарию мониторинга.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Отчёт модуля 8 недоступен",
                    "Не удалось загрузить статус или результат углублённого анализа.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_module_eight_audit_page(&scenario, &audit)),
    )
        .into_response()
}

pub async fn module_nine_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 9 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий и получите хотя бы один анализ карточки.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 9 недоступен",
                    "Не удалось загрузить сценарий для истории расширенных отчётов.",
                )),
            )
                .into_response();
        }
    };

    if sync_module_nine_reports(&state.pool, &state.report_storage_dir, scenario_id)
        .await
        .is_err()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(render_not_found_page(
                "История отчётов недоступна",
                "Не удалось подготовить расширенную историю запросов. Попробуйте открыть модуль позже.",
            )),
        )
            .into_response();
    }

    let reports = match fetch_module_nine_reports(&state.pool, scenario_id).await {
        Ok(rows) => rows,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "История отчётов недоступна",
                    "Не удалось загрузить список готовых расширенных отчётов.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_module_nine_page(&scenario, &reports)),
    )
        .into_response()
}

pub async fn module_ten_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 10 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в PRO-модуле.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 10 недоступен",
                    "Не удалось загрузить сценарий фонового мониторинга.",
                )),
            )
                .into_response();
        }
    };

    let events = fetch_pro_monitoring_events(&state.pool, scenario_id)
        .await
        .unwrap_or_default();
    let audits = fetch_module_eight_audits(&state.pool, scenario_id)
        .await
        .unwrap_or_default();
    let _ = sync_module_nine_reports(&state.pool, &state.report_storage_dir, scenario_id).await;
    let reports = fetch_module_nine_reports(&state.pool, scenario_id)
        .await
        .unwrap_or_default();

    (
        StatusCode::OK,
        Html(render_module_ten_page(
            &scenario, &events, &audits, &reports,
        )),
    )
        .into_response()
}

pub async fn module_eleven_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 11 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в PRO-модуле.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 11 недоступен",
                    "Не удалось загрузить переход к новому анализу карточки.",
                )),
            )
                .into_response();
        }
    };

    (StatusCode::OK, Html(render_module_eleven_page(&scenario))).into_response()
}

pub async fn module_twelve_page(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 12 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в PRO-модуле.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 12 недоступен",
                    "Не удалось загрузить страницу нового анализа.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_module_twelve_page(
            &scenario,
            &ModuleTwelveForm {
                source_url: String::new(),
                business_category: scenario.business_category.clone(),
            },
            &[],
        )),
    )
        .into_response()
}

pub async fn module_twelve_submit(
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
    Form(form): Form<ModuleTwelveForm>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 12 не найден",
                    "Сценарий мониторинга не найден. Сначала создайте сценарий в PRO-модуле.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 12 недоступен",
                    "Не удалось загрузить сценарий для нового анализа.",
                )),
            )
                .into_response();
        }
    };

    let mut issues = Vec::new();

    let category = collapse_whitespace(&form.business_category);
    if category.is_empty() {
        issues.push("Выберите категорию бизнеса перед запуском анализа.".to_string());
    }

    let parsed_url = match parse_business_profile_url(&form.source_url) {
        Ok(parsed_url) => {
            if matches!(parsed_url.source_platform, MapPlatform::TwoGis) {
                issues.push(
                    "Для нового анализа сейчас поддерживаются только Яндекс.Карты и Google Maps."
                        .to_string(),
                );
                None
            } else {
                Some(parsed_url)
            }
        }
        Err(error) => {
            issues.push(error);
            None
        }
    };

    let parsed_url = if let Some(parsed_url) = parsed_url {
        match resolve_business_profile_url(parsed_url).await {
            Ok(parsed_url) => Some(parsed_url),
            Err(issue) => {
                issues.push(format!("{} {}", issue.title, issue.detail));
                None
            }
        }
    } else {
        None
    };

    if !issues.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(render_module_twelve_page(&scenario, &form, &issues)),
        )
            .into_response();
    }

    if category != scenario.business_category {
        let _ = sqlx::query(
            r#"
            update pro_monitoring_scenarios
            set business_category = $2,
                updated_at = now()
            where id = $1
            "#,
        )
        .bind(scenario_id)
        .bind(&category)
        .execute(&state.pool)
        .await;
    }

    let parsed_url = parsed_url.expect("module 12 url should be parsed after validation");

    let (audit_id,) = match sqlx::query_as::<_, (Uuid,)>(
        r#"
        insert into module_eight_audits (
            scenario_id,
            source_platform,
            source_url,
            canonical_url,
            request_status,
            current_stage,
            progress_percent,
            status_message
        )
        values ($1, $2, $3, $4, 'queued', 'validate_request', 5, 'Готовим запуск анализа')
        returning id
        "#,
    )
    .bind(scenario_id)
    .bind(parsed_url.source_platform.as_db_value())
    .bind(&parsed_url.source_url)
    .bind(&parsed_url.canonical_url)
    .fetch_one(&state.pool)
    .await
    {
        Ok(row) => row,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_module_twelve_page(
                    &scenario,
                    &form,
                    &[
                        "Не удалось сохранить новый анализ. Попробуйте ещё раз немного позже."
                            .to_string(),
                    ],
                )),
            )
                .into_response();
        }
    };

    tokio::spawn(run_module_eight_audit(
        state.pool.clone(),
        state.redis.clone(),
        scenario.id,
        audit_id,
        parsed_url.source_platform,
        parsed_url.canonical_url,
    ));

    Redirect::to(&format!(
        "/pricing/pro/monitor/{scenario_id}/module-8/{audit_id}"
    ))
    .into_response()
}

pub async fn module_nine_report_page(
    State(state): State<AppState>,
    Path((scenario_id, report_id)): Path<(Uuid, Uuid)>,
) -> Response {
    let scenario = match fetch_pro_monitoring_scenario(&state.pool, scenario_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Модуль 9 не найден",
                    "Сценарий мониторинга не найден. Запустите его заново из PRO-модуля.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 9 недоступен",
                    "Не удалось загрузить сценарий для расширенного отчёта.",
                )),
            )
                .into_response();
        }
    };

    if sync_module_nine_reports(&state.pool, &state.report_storage_dir, scenario_id)
        .await
        .is_err()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(render_not_found_page(
                "Отчёт недоступен",
                "Не удалось подготовить PDF и данные для этого отчёта.",
            )),
        )
            .into_response();
    }

    let report = match fetch_module_nine_report(&state.pool, scenario_id, report_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Отчёт не найден",
                    "Расширенный отчёт не найден или не относится к выбранному сценарию.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Отчёт недоступен",
                    "Не удалось загрузить расширенный отчёт. Попробуйте позже.",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Html(render_module_nine_report_page(&scenario, &report)),
    )
        .into_response()
}

pub async fn download_module_nine_pdf(
    State(state): State<AppState>,
    Path((scenario_id, report_id)): Path<(Uuid, Uuid)>,
) -> Response {
    if sync_module_nine_reports(&state.pool, &state.report_storage_dir, scenario_id)
        .await
        .is_err()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(render_not_found_page(
                "PDF недоступен",
                "Не удалось подготовить PDF-файл для скачивания.",
            )),
        )
            .into_response();
    }

    let report = match fetch_module_nine_report(&state.pool, scenario_id, report_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "PDF не найден",
                    "Запрошенный PDF не найден или не относится к выбранному сценарию.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "PDF недоступен",
                    "Не удалось загрузить метаданные PDF-файла.",
                )),
            )
                .into_response();
        }
    };

    let file_path = match report.pdf_disk_path.as_deref() {
        Some(path) if report.pdf_status == "ready" => path,
        _ => {
            return (
                StatusCode::CONFLICT,
                Html(render_not_found_page(
                    "PDF ещё готовится",
                    "Файл ещё не собран или был повреждён. Обновите страницу отчёта через несколько секунд.",
                )),
            )
                .into_response();
        }
    };

    let bytes = match tokio::fs::read(file_path).await {
        Ok(content) => content,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "PDF недоступен",
                    "Файл отчёта не удалось прочитать с диска. Попробуйте пересобрать отчёт позже.",
                )),
            )
                .into_response();
        }
    };

    let file_name = report
        .pdf_file_name
        .unwrap_or_else(|| format!("module-9-report-{}.pdf", report.report_id));

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{file_name}\""),
            ),
        ],
        Bytes::from(bytes),
    )
        .into_response()
}

async fn validate_analysis_submission(
    pool: &PgPool,
    user_id: Uuid,
    parsed_url: &ParsedBusinessUrl,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    match has_duplicate_business_profile(pool, user_id, &parsed_url.canonical_url).await {
        Ok(true) => issues.push(validation_issue(
            "Карточка уже есть в истории",
            "Такая карточка уже есть в истории поиска текущего пользователя. Откройте существующий анализ вместо повторного запуска.",
            false,
        )),
        Ok(false) => {}
        Err(_) => issues.push(validation_issue(
            "Попробуйте позже",
            "Не удалось проверить историю поиска пользователя. Повторите попытку немного позже.",
            false,
        )),
    }

    issues
}

async fn require_authenticated_user(
    pool: &PgPool,
    headers: &HeaderMap,
    expected_user_id: Uuid,
) -> Result<RegisteredUser, Response> {
    match fetch_session_user(pool, headers).await {
        Ok(Some(user)) if user.id == expected_user_id => Ok(user),
        Ok(Some(_)) => Err(redirect_with_cookie("/login", &expire_session_cookie())),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(_) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Html(render_not_found_page(
                "Авторизация недоступна",
                "Не удалось проверить текущую сессию. Попробуйте снова немного позже.",
            )),
        )
            .into_response()),
    }
}
async fn render_cabinet_response(
    state: AppState,
    user_id: Uuid,
    form: CabinetAnalysisForm,
    issues: &[ValidationIssue],
    status: StatusCode,
) -> Response {
    let user = match fetch_registered_user(&state.pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Кабинет не найден",
                    "Пользователь не найден. Проверьте ссылку на кабинет или зарегистрируйтесь заново.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Ошибка чтения",
                    "Не удалось загрузить кабинет. Попробуйте обновить страницу позже.",
                )),
            )
                .into_response();
        }
    };

    let records = fetch_user_analysis_requests(&state.pool, user_id)
        .await
        .unwrap_or_default();

    render_cabinet_page(status, &user, &form, issues, &records).into_response()
}
async fn render_subscription_response(
    state: AppState,
    user_id: Uuid,
    feedback: Option<SubscriptionFeedback>,
    status: StatusCode,
) -> Response {
    let user = match fetch_registered_user(&state.pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html(render_not_found_page(
                    "Подписка не найдена",
                    "Пользователь не найден. Проверьте ссылку на кабинет или зарегистрируйтесь заново.",
                )),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render_not_found_page(
                    "Модуль 13 недоступен",
                    "Не удалось загрузить управление подпиской. Попробуйте обновить страницу позже.",
                )),
            )
                .into_response();
        }
    };

    (
        status,
        Html(render_subscription_page(&user, feedback.as_ref())),
    )
        .into_response()
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let query = collapse_whitespace(&params.q);
    let rows: Vec<GeoObject> = sqlx::query_as(
        "SELECT name, object_type, city FROM geo_objects
         WHERE lower(name) LIKE lower($1) OR lower(city) LIKE lower($1)
         ORDER BY name LIMIT 50",
    )
    .bind(format!("%{}%", query))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let analysis = analyze_query(&query, &rows);
    let result_rows = rows
        .iter()
        .map(|row| {
            format!(
                r#"<tr><td>{name}</td><td>{object_type}</td><td>{city}</td></tr>"#,
                name = html_escape(&row.name),
                object_type = html_escape(&row.object_type),
                city = html_escape(&row.city),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let empty_note = if rows.is_empty() {
        format!(
            r#"<p style="color:#617168;">По запросу «{}» ничего не найдено.</p>"#,
            html_escape(&query)
        )
    } else {
        String::new()
    };

    Html(format!(
        r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Поиск: {query_escaped} — Geonetix</title>
  <style>
    body{{margin:0;padding:24px;font-family:"Segoe UI","Helvetica Neue",sans-serif;background:#f7f4ec;color:#1d2b24;}}
    main{{max-width:860px;margin:0 auto;}}
    section{{background:#fff;border:1px solid #d8dfd5;border-radius:18px;padding:24px;margin-bottom:16px;}}
    h1{{margin:0 0 8px;font-size:1.3rem;}}
    p{{margin:0 0 10px;line-height:1.6;color:#617168;}}
    table{{width:100%;border-collapse:collapse;font-size:.9rem;}}
    th,td{{padding:8px 12px;border-bottom:1px solid #e8ede9;text-align:left;}}
    th{{color:#617168;font-weight:600;}}
    a{{color:#204f3f;}}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>Результаты поиска: «{query_escaped}»</h1>
      <p>Найдено объектов: {result_count}. Уверенность: {confidence_label}.</p>
      <p style="font-size:.85rem;">{confidence_note}</p>
      <p style="margin-top:8px;"><a href="/">← На главную</a></p>
    </section>
    <section>
      {empty_note}
      <table>
        <thead><tr><th>Название</th><th>Тип</th><th>Город</th></tr></thead>
        <tbody>{result_rows}</tbody>
      </table>
    </section>
  </main>
</body>
</html>"#,
        query_escaped = html_escape(&query),
        result_count = analysis.result_count,
        confidence_label = html_escape(analysis.confidence_label),
        confidence_note = html_escape(analysis.confidence_note),
        empty_note = empty_note,
        result_rows = result_rows,
    ))
}
