//! System administration query routes.

use axum::extract::Extension;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch, post},
};
use cuba_shared::{
    ApiResponse, AppError, AppResult, AppState, CurrentUser, Page, audit_category_for_action,
    audit_module_for_event, map_master_data_db_error,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder, Row};

const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 100;

/// Build system administration routes.
pub fn router() -> Router<AppState> {
    let audit_routes = Router::new()
        .route("/api/system/audit-logs", get(list_audit_logs))
        .layer(axum::middleware::from_fn(|req, next| {
            crate::middleware::require_permission("audit:read", req, next)
        }));

    let admin_read_routes = Router::new()
        .route("/api/system/users", get(list_system_users))
        .route("/api/system/roles", get(list_system_roles))
        .layer(axum::middleware::from_fn(|req, next| {
            crate::middleware::require_role("ADMIN", req, next)
        }));

    let param_read_routes = Router::new()
        .route("/api/system/params", get(list_system_params))
        .route("/api/system/params/{param_key}", get(get_system_param))
        .layer(axum::middleware::from_fn(|req, next| {
            crate::middleware::require_permission("system-param:read", req, next)
        }));

    let param_write_routes = Router::new()
        .route("/api/system/params", post(create_system_param))
        .route("/api/system/params/{param_key}", patch(update_system_param))
        .layer(axum::middleware::from_fn(|req, next| {
            crate::middleware::require_permission("system-param:write", req, next)
        }));

    Router::new()
        .merge(audit_routes)
        .merge(admin_read_routes)
        .merge(param_read_routes)
        .merge(param_write_routes)
}

#[derive(Debug, Deserialize)]
struct AuditLogQuery {
    page: Option<u64>,
    page_size: Option<u64>,
    keyword: Option<String>,
    action: Option<String>,
    category: Option<String>,
    module: Option<String>,
    table_name: Option<String>,
    record_id: Option<String>,
    user_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditLogRecord {
    id: i64,
    created_at: String,
    username: Option<String>,
    user_id: Option<String>,
    action: String,
    category: String,
    module: String,
    table_name: Option<String>,
    record_id: Option<String>,
    ip_address: Option<String>,
    old_data: Option<Value>,
    new_data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SystemUserQuery {
    page: Option<u64>,
    page_size: Option<u64>,
    keyword: Option<String>,
    role_id: Option<String>,
    is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SystemUserRecord {
    user_id: String,
    username: String,
    full_name: Option<String>,
    email: Option<String>,
    role_id: Option<String>,
    roles: Vec<String>,
    permissions_count: i64,
    is_active: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct SystemRoleQuery {
    page: Option<u64>,
    page_size: Option<u64>,
    keyword: Option<String>,
}

#[derive(Debug, Serialize)]
struct SystemRoleRecord {
    role_id: String,
    role_name: String,
    description: Option<String>,
    user_count: i64,
    permission_count: i64,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct SystemParamQuery {
    page: Option<u64>,
    page_size: Option<u64>,
    keyword: Option<String>,
    param_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertSystemParamRequest {
    param_key: Option<String>,
    param_value: String,
    param_type: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct SystemParamRecord {
    param_key: String,
    param_value: String,
    param_type: String,
    description: Option<String>,
    updated_by: Option<String>,
    updated_at: String,
}

async fn list_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<AuditLogQuery>,
) -> AppResult<Json<ApiResponse<Page<AuditLogRecord>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let offset_i64 =
        i64::try_from(offset).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;
    let limit_i64 =
        i64::try_from(page_size).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;

    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(*) AS total
        FROM sys.sys_audit_log l
        LEFT JOIN sys.sys_users u ON u.user_id = l.user_id
        WHERE 1 = 1
        "#,
    );
    append_audit_filters(&mut count_builder, &query);

    let count_row = count_builder
        .build()
        .fetch_one(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let total_i64: i64 = count_row.get("total");
    let total = u64::try_from(total_i64).map_or(0, |value| value);

    let mut data_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            l.id,
            l.created_at::text AS created_at,
            u.username,
            l.user_id::text AS user_id,
            l.action,
            l.table_name,
            l.record_id,
            l.ip_address::text AS ip_address,
            l.old_data,
            l.new_data
        FROM sys.sys_audit_log l
        LEFT JOIN sys.sys_users u ON u.user_id = l.user_id
        WHERE 1 = 1
        "#,
    );
    append_audit_filters(&mut data_builder, &query);
    data_builder
        .push(" ORDER BY l.id DESC LIMIT ")
        .push_bind(limit_i64)
        .push(" OFFSET ")
        .push_bind(offset_i64);

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let action: String = row.get("action");
            let table_name: Option<String> = row.get("table_name");
            let category = audit_category_for_action(&action).as_str().to_string();
            let module = audit_module_for_event(&action, table_name.as_deref()).to_string();

            AuditLogRecord {
                id: row.get("id"),
                created_at: row.get("created_at"),
                username: row.get("username"),
                user_id: row.get("user_id"),
                action,
                category,
                module,
                table_name,
                record_id: row.get("record_id"),
                ip_address: row.get("ip_address"),
                old_data: row.get("old_data"),
                new_data: row.get("new_data"),
            }
        })
        .collect();

    Ok(Json(ApiResponse::ok(Page::new(
        items, total, page, page_size,
    ))))
}

async fn list_system_users(
    State(state): State<AppState>,
    Query(query): Query<SystemUserQuery>,
) -> AppResult<Json<ApiResponse<Page<SystemUserRecord>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let offset_i64 =
        i64::try_from(offset).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;
    let limit_i64 =
        i64::try_from(page_size).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;

    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(*) AS total
        FROM sys.sys_users u
        WHERE 1 = 1
        "#,
    );
    append_system_user_filters(&mut count_builder, &query);

    let count_row = count_builder
        .build()
        .fetch_one(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let total_i64: i64 = count_row.get("total");
    let total = u64::try_from(total_i64).map_or(0, |value| value);

    let mut data_builder = QueryBuilder::<Postgres>::new(
        r#"
        WITH all_user_roles AS (
            SELECT user_id, role_id FROM sys.sys_user_roles
            UNION
            SELECT user_id, role_id FROM sys.sys_users WHERE role_id IS NOT NULL
        ),
        role_agg AS (
            SELECT user_id, array_agg(DISTINCT role_id ORDER BY role_id) AS roles
            FROM all_user_roles
            GROUP BY user_id
        ),
        permission_agg AS (
            SELECT
                u.user_id,
                COUNT(DISTINCT p.permission_code)::bigint AS permissions_count
            FROM sys.sys_users u
            LEFT JOIN all_user_roles ur ON ur.user_id = u.user_id
            LEFT JOIN sys.sys_user_permissions p
              ON p.granted = true
             AND (p.expires_at IS NULL OR p.expires_at > NOW())
             AND (p.user_id = u.user_id OR p.role_id = ur.role_id)
            GROUP BY u.user_id
        )
        SELECT
            u.user_id::text AS user_id,
            u.username,
            u.full_name,
            u.email,
            u.role_id,
            COALESCE(r.roles, ARRAY[]::varchar[]) AS roles,
            COALESCE(p.permissions_count, 0)::bigint AS permissions_count,
            u.is_active,
            u.created_at::text AS created_at,
            u.updated_at::text AS updated_at
        FROM sys.sys_users u
        LEFT JOIN role_agg r ON r.user_id = u.user_id
        LEFT JOIN permission_agg p ON p.user_id = u.user_id
        WHERE 1 = 1
        "#,
    );
    append_system_user_filters(&mut data_builder, &query);
    data_builder
        .push(" ORDER BY u.username ASC LIMIT ")
        .push_bind(limit_i64)
        .push(" OFFSET ")
        .push_bind(offset_i64);

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let items = rows
        .into_iter()
        .map(|row| SystemUserRecord {
            user_id: row.get("user_id"),
            username: row.get("username"),
            full_name: row.get("full_name"),
            email: row.get("email"),
            role_id: row.get("role_id"),
            roles: row.get("roles"),
            permissions_count: row.get("permissions_count"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(Json(ApiResponse::ok(Page::new(
        items, total, page, page_size,
    ))))
}

async fn list_system_roles(
    State(state): State<AppState>,
    Query(query): Query<SystemRoleQuery>,
) -> AppResult<Json<ApiResponse<Page<SystemRoleRecord>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let offset_i64 =
        i64::try_from(offset).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;
    let limit_i64 =
        i64::try_from(page_size).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;

    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(*) AS total
        FROM sys.sys_roles r
        WHERE 1 = 1
        "#,
    );
    append_system_role_filters(&mut count_builder, &query);

    let count_row = count_builder
        .build()
        .fetch_one(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let total_i64: i64 = count_row.get("total");
    let total = u64::try_from(total_i64).map_or(0, |value| value);

    let mut data_builder = QueryBuilder::<Postgres>::new(
        r#"
        WITH all_user_roles AS (
            SELECT user_id, role_id FROM sys.sys_user_roles
            UNION
            SELECT user_id, role_id FROM sys.sys_users WHERE role_id IS NOT NULL
        )
        SELECT
            r.role_id,
            r.role_name,
            r.description,
            COUNT(DISTINCT ur.user_id)::bigint AS user_count,
            COUNT(DISTINCT p.permission_code)::bigint AS permission_count,
            r.created_at::text AS created_at
        FROM sys.sys_roles r
        LEFT JOIN all_user_roles ur ON ur.role_id = r.role_id
        LEFT JOIN sys.sys_user_permissions p
          ON p.role_id = r.role_id
         AND p.granted = true
         AND (p.expires_at IS NULL OR p.expires_at > NOW())
        WHERE 1 = 1
        "#,
    );
    append_system_role_filters(&mut data_builder, &query);
    data_builder
        .push(" GROUP BY r.role_id, r.role_name, r.description, r.created_at")
        .push(" ORDER BY r.role_id ASC LIMIT ")
        .push_bind(limit_i64)
        .push(" OFFSET ")
        .push_bind(offset_i64);

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let items = rows
        .into_iter()
        .map(|row| SystemRoleRecord {
            role_id: row.get("role_id"),
            role_name: row.get("role_name"),
            description: row.get("description"),
            user_count: row.get("user_count"),
            permission_count: row.get("permission_count"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(ApiResponse::ok(Page::new(
        items, total, page, page_size,
    ))))
}

async fn list_system_params(
    State(state): State<AppState>,
    Query(query): Query<SystemParamQuery>,
) -> AppResult<Json<ApiResponse<Page<SystemParamRecord>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let offset_i64 =
        i64::try_from(offset).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;
    let limit_i64 =
        i64::try_from(page_size).map_err(|_| AppError::Validation("分页参数过大".to_string()))?;

    let mut count_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT COUNT(*) AS total
        FROM sys.sys_system_params
        WHERE 1 = 1
        "#,
    );
    append_system_param_filters(&mut count_builder, &query);

    let count_row = count_builder
        .build()
        .fetch_one(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;
    let total_i64: i64 = count_row.get("total");
    let total = u64::try_from(total_i64).map_or(0, |value| value);

    let mut data_builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at::text AS updated_at
        FROM sys.sys_system_params
        WHERE 1 = 1
        "#,
    );
    append_system_param_filters(&mut data_builder, &query);
    data_builder
        .push(" ORDER BY param_key ASC LIMIT ")
        .push_bind(limit_i64)
        .push(" OFFSET ")
        .push_bind(offset_i64);

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(map_system_param_db_error)?;

    let items = rows.into_iter().map(system_param_from_row).collect();

    Ok(Json(ApiResponse::ok(Page::new(
        items, total, page, page_size,
    ))))
}

async fn get_system_param(
    State(state): State<AppState>,
    Path(param_key): Path<String>,
) -> AppResult<Json<ApiResponse<SystemParamRecord>>> {
    let row = sqlx::query(
        r#"
        SELECT
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at::text AS updated_at
        FROM sys.sys_system_params
        WHERE param_key = $1
        "#,
    )
    .bind(param_key.trim())
    .fetch_optional(&state.db_pool)
    .await
    .map_err(map_system_param_db_error)?
    .ok_or_else(|| AppError::NotFound("系统参数不存在".to_string()))?;

    Ok(Json(ApiResponse::ok(system_param_from_row(row))))
}

async fn create_system_param(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<UpsertSystemParamRequest>,
) -> AppResult<Json<ApiResponse<SystemParamRecord>>> {
    let param_key = clean_required(request.param_key.as_deref(), "参数键")?;
    let param_type = normalize_param_type(request.param_type.as_deref())?;
    validate_param_value(&request.param_value, &param_type)?;

    let row = sqlx::query(
        r#"
        INSERT INTO sys.sys_system_params (
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at::text AS updated_at
        "#,
    )
    .bind(&param_key)
    .bind(request.param_value.trim())
    .bind(&param_type)
    .bind(clean_optional(request.description.as_deref()))
    .bind(&user.username)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_system_param_db_error)?;

    let record = system_param_from_row(row);
    cuba_shared::write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "SYSTEM_PARAM_CREATE",
        Some("sys.sys_system_params"),
        Some(&record.param_key),
        Some(json!({
            "param_key": record.param_key,
            "param_type": record.param_type,
            "description": record.description
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(record)))
}

async fn update_system_param(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(param_key): Path<String>,
    Json(request): Json<UpsertSystemParamRequest>,
) -> AppResult<Json<ApiResponse<SystemParamRecord>>> {
    let param_key = clean_required(Some(param_key.as_str()), "参数键")?;

    let old_row = sqlx::query(
        r#"
        SELECT
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at::text AS updated_at
        FROM sys.sys_system_params
        WHERE param_key = $1
        "#,
    )
    .bind(&param_key)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(map_system_param_db_error)?
    .ok_or_else(|| AppError::NotFound("系统参数不存在".to_string()))?;
    let old_record = system_param_from_row(old_row);

    let param_type = match request.param_type.as_deref().and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    }) {
        Some(value) => normalize_param_type(Some(value))?,
        None => old_record.param_type.clone(),
    };
    validate_param_value(&request.param_value, &param_type)?;

    let row = sqlx::query(
        r#"
        UPDATE sys.sys_system_params
        SET
            param_value = $2,
            param_type = $3,
            description = $4,
            updated_by = $5,
            updated_at = NOW()
        WHERE param_key = $1
        RETURNING
            param_key,
            param_value,
            param_type,
            description,
            updated_by,
            updated_at::text AS updated_at
        "#,
    )
    .bind(&param_key)
    .bind(request.param_value.trim())
    .bind(&param_type)
    .bind(clean_optional(request.description.as_deref()))
    .bind(&user.username)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_system_param_db_error)?;

    let record = system_param_from_row(row);
    cuba_shared::write_audit_change(
        &state.db_pool,
        Some(user.user_id),
        "SYSTEM_PARAM_UPDATE",
        "sys.sys_system_params",
        &record.param_key,
        Some(json!({
            "param_value": old_record.param_value,
            "param_type": old_record.param_type,
            "description": old_record.description
        })),
        Some(json!({
            "param_value": record.param_value,
            "param_type": record.param_type,
            "description": record.description
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(record)))
}

fn append_audit_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &AuditLogQuery) {
    if let Some(keyword) = clean(&query.keyword) {
        let pattern = format!("%{keyword}%");
        builder
            .push(" AND (l.action ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(l.table_name, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(l.record_id, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(u.username, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(l.user_id::text, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(l.ip_address::text, '') ILIKE ")
            .push_bind(pattern)
            .push(")");
    }

    if let Some(action) = clean(&query.action) {
        builder.push(" AND l.action = ").push_bind(action);
    }

    if let Some(category) = clean(&query.category) {
        let actions = actions_for_category(&category);
        if !actions.is_empty() {
            builder
                .push(" AND l.action = ANY(")
                .push_bind(actions)
                .push(")");
        }
    }

    if let Some(module) = clean(&query.module) {
        append_module_filter(builder, &module);
    }

    if let Some(table_name) = clean(&query.table_name) {
        builder.push(" AND l.table_name = ").push_bind(table_name);
    }

    if let Some(record_id) = clean(&query.record_id) {
        builder.push(" AND l.record_id = ").push_bind(record_id);
    }

    if let Some(user_id) = clean(&query.user_id) {
        builder.push(" AND l.user_id::text = ").push_bind(user_id);
    }

    if let Some(date_from) = clean(&query.date_from) {
        builder
            .push(" AND l.created_at >= ")
            .push_bind(date_from)
            .push("::timestamptz");
    }

    if let Some(date_to) = clean(&query.date_to) {
        builder
            .push(" AND l.created_at <= ")
            .push_bind(date_to)
            .push("::timestamptz");
    }
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn clean_required(value: Option<&str>, field: &str) -> AppResult<String> {
    clean_optional(value).ok_or_else(|| AppError::Validation(format!("{field}不能为空")))
}

fn normalize_param_type(value: Option<&str>) -> AppResult<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("string");
    match value {
        "string" | "number" | "boolean" | "json" => Ok(value.to_string()),
        _ => Err(AppError::Validation(
            "参数类型必须是 string、number、boolean 或 json".to_string(),
        )),
    }
}

fn validate_param_value(value: &str, param_type: &str) -> AppResult<()> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::Validation("参数值不能为空".to_string()));
    }

    match param_type {
        "number" => {
            value
                .parse::<f64>()
                .map_err(|_| AppError::Validation("number 类型参数值必须是数字".to_string()))?;
        }
        "boolean" => match value {
            "true" | "false" => {}
            _ => {
                return Err(AppError::Validation(
                    "boolean 类型参数值必须是 true 或 false".to_string(),
                ));
            }
        },
        "json" => {
            serde_json::from_str::<Value>(value)
                .map_err(|_| AppError::Validation("json 类型参数值必须是合法 JSON".to_string()))?;
        }
        "string" => {}
        _ => {
            return Err(AppError::Validation(
                "参数类型必须是 string、number、boolean 或 json".to_string(),
            ));
        }
    }

    Ok(())
}

fn append_system_user_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &SystemUserQuery) {
    if let Some(keyword) = clean(&query.keyword) {
        let pattern = format!("%{keyword}%");
        builder
            .push(" AND (u.username ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(u.full_name, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(u.email, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR u.user_id::text ILIKE ")
            .push_bind(pattern)
            .push(")");
    }

    if let Some(role_id) = clean(&query.role_id) {
        builder
            .push(" AND (u.role_id = ")
            .push_bind(role_id.clone())
            .push(" OR EXISTS (SELECT 1 FROM sys.sys_user_roles ur_filter WHERE ur_filter.user_id = u.user_id AND ur_filter.role_id = ")
            .push_bind(role_id)
            .push("))");
    }

    if let Some(is_active) = query.is_active {
        builder.push(" AND u.is_active = ").push_bind(is_active);
    }
}

fn append_system_role_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &SystemRoleQuery) {
    if let Some(keyword) = clean(&query.keyword) {
        let pattern = format!("%{keyword}%");
        builder
            .push(" AND (r.role_id ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR r.role_name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(r.description, '') ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
}

fn append_system_param_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &SystemParamQuery) {
    if let Some(keyword) = clean(&query.keyword) {
        let pattern = format!("%{keyword}%");
        builder
            .push(" AND (param_key ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR param_value ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(description, '') ILIKE ")
            .push_bind(pattern)
            .push(")");
    }

    if let Some(param_type) = clean(&query.param_type) {
        builder.push(" AND param_type = ").push_bind(param_type);
    }
}

fn system_param_from_row(row: sqlx::postgres::PgRow) -> SystemParamRecord {
    SystemParamRecord {
        param_key: row.get("param_key"),
        param_value: row.get("param_value"),
        param_type: row.get("param_type"),
        description: row.get("description"),
        updated_by: row.get("updated_by"),
        updated_at: row.get("updated_at"),
    }
}

fn map_system_param_db_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.is_unique_violation() {
            return AppError::business("SYSTEM_PARAM_ALREADY_EXISTS", "系统参数已存在");
        }
    }

    map_master_data_db_error(error)
}

fn actions_for_category(category: &str) -> Vec<&'static str> {
    match category.trim().to_ascii_uppercase().as_str() {
        "LOGIN" => vec!["LOGIN", "LOGIN_FAILED", "REFRESH_TOKEN"],
        "DATA_CHANGE" => vec![
            "INSERT",
            "UPDATE",
            "DELETE",
            "MASTER_DATA_CREATE",
            "MASTER_DATA_UPDATE",
            "MASTER_DATA_ACTIVATE",
            "MASTER_DATA_DEACTIVATE",
            "MASTER_DATA_DELETE",
            "MASTER_DATA_SET_PRIMARY",
        ],
        "POSTING" => vec![
            "INVENTORY_POST",
            "INVENTORY_TRANSFER",
            "INVENTORY_COUNT_POST",
            "PURCHASE_RECEIPT_POST",
            "SALES_SHIPMENT_POST",
            "PRODUCTION_COMPLETE_POST",
        ],
        "QUALITY" => vec![
            "QUALITY_INSPECTION_LOT_CREATE",
            "QUALITY_INSPECTION_RESULT_ADD",
            "QUALITY_DECISION",
            "QUALITY_BATCH_FREEZE",
            "QUALITY_BATCH_UNFREEZE",
            "QUALITY_BATCH_SCRAP",
        ],
        "SYSTEM" => vec!["INIT_SCHEMA"],
        _ => Vec::new(),
    }
}

fn append_module_filter(builder: &mut QueryBuilder<'_, Postgres>, module: &str) {
    match module.trim().to_ascii_lowercase().as_str() {
        "auth" => {
            builder
                .push(" AND l.action = ANY(")
                .push_bind(vec!["LOGIN", "LOGIN_FAILED", "REFRESH_TOKEN"])
                .push(")");
        }
        "master-data" => {
            builder.push(" AND (l.table_name LIKE 'mdm.%' OR l.action LIKE 'MASTER_DATA_%')");
        }
        "inventory" => {
            builder.push(" AND (l.table_name LIKE 'wms.wms_inventory%' OR l.table_name = 'wms.wms_transactions' OR l.table_name = 'wms.wms_batches' OR l.action LIKE 'INVENTORY_%')");
        }
        "quality" => {
            builder.push(" AND (l.table_name LIKE '%quality%' OR l.table_name LIKE '%inspection%' OR l.action LIKE 'QUALITY_%')");
        }
        "purchase" => {
            builder.push(" AND (l.table_name LIKE '%purchase%' OR l.action LIKE 'PURCHASE_%')");
        }
        "sales" => {
            builder.push(" AND (l.table_name LIKE '%sales%' OR l.action LIKE 'SALES_%')");
        }
        "production" => {
            builder.push(" AND (l.table_name LIKE '%production%' OR l.action LIKE 'PRODUCTION_%')");
        }
        "system" => {
            builder.push(" AND l.table_name LIKE 'sys.%'");
        }
        _ => {}
    }
}
