use super::dto::{
    AuthResponse, CurrentUserPermissionsDto, CurrentUserRolesDto, LoginRequest, LoginResponse,
    RefreshTokenRequest, UserInfoDto,
};
use crate::application::{
    current_user_from_fresh_grants, ensure_refresh_token_usable, ensure_refresh_user_enabled,
    issue_refresh_token, parse_refresh_token, verify_refresh_secret,
};
use crate::infrastructure::PostgresAuthRepository;
use axum::{
    Json,
    extract::{Extension, State},
    http::HeaderMap,
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState, CurrentUser};
use time::OffsetDateTime;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse {
        module: "auth",
        status: "ready",
    })))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());
    let client_ip = extract_client_ip(&headers);

    // 1. 查用户;不存在时也跑一次 dummy verify 防时序攻击
    let user_opt = auth_repo.find_user_by_username(&req.username).await?;

    let user = match user_opt {
        Some(u) => u,
        None => {
            run_dummy_password_verify();
            // 失败也写一条审计,user_id 为空,action 为 LOGIN_FAILED
            let _ = auth_repo
                .write_audit_log(None, "LOGIN_FAILED", client_ip)
                .await;
            return Err(AppError::Unauthorized("用户名或密码错误".to_string()));
        }
    };

    // 2. 拉取角色和权限(写进 JWT)
    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    // 3. 跑业务用例:验密码 + 签发 token
    let login_use_case = crate::application::LoginUseCase::new(
        state.jwt_secret.clone(),
        state.jwt_issuer.clone(),
        state.jwt_expires_seconds,
    );

    let exec_result = login_use_case.execute(&user, &req.password, roles, permissions);

    let (token, current_user) = match exec_result {
        Ok(v) => v,
        Err(err) => {
            // 失败审计(密码错 / 用户禁用都走这里)
            let _ = auth_repo
                .write_audit_log(Some(user.user_id), "LOGIN_FAILED", client_ip)
                .await;
            return Err(err);
        }
    };

    let refresh_token = issue_refresh_token(state.jwt_refresh_expires_seconds)?;
    auth_repo
        .save_refresh_token(
            refresh_token.token_id,
            user.user_id,
            &refresh_token.selector,
            &refresh_token.token_hash,
            refresh_token.expires_at,
        )
        .await?;

    // 4. 成功审计
    let _ = auth_repo
        .write_audit_log(Some(user.user_id), "LOGIN", client_ip)
        .await;

    let response = build_login_response(&state, token, refresh_token.token, current_user);

    Ok(Json(ApiResponse::ok(response)))
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RefreshTokenRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());
    let client_ip = extract_client_ip(&headers);
    let parsed = parse_refresh_token(&req.refresh_token)?;

    let stored = auth_repo
        .find_refresh_token_by_selector(&parsed.selector)
        .await?
        .ok_or_else(refresh_token_invalid)?;

    ensure_refresh_token_usable(&stored, OffsetDateTime::now_utc())?;

    verify_refresh_secret(&parsed, &stored.token_hash)?;

    let user = auth_repo
        .find_user_by_id(stored.user_id)
        .await?
        .ok_or_else(refresh_token_invalid)?;

    ensure_refresh_user_enabled(&user)?;

    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    let login_use_case = crate::application::LoginUseCase::new(
        state.jwt_secret.clone(),
        state.jwt_issuer.clone(),
        state.jwt_expires_seconds,
    );

    let access_token = login_use_case.issue_access_token(&user, &roles, &permissions)?;
    let current_user = current_user_from_fresh_grants(&user, roles, permissions);

    let next_refresh_token = issue_refresh_token(state.jwt_refresh_expires_seconds)?;
    auth_repo
        .rotate_refresh_token(
            stored.token_id,
            next_refresh_token.token_id,
            user.user_id,
            &next_refresh_token.selector,
            &next_refresh_token.token_hash,
            next_refresh_token.expires_at,
        )
        .await?;

    let _ = auth_repo
        .write_audit_log(Some(user.user_id), "REFRESH_TOKEN", client_ip)
        .await;

    let response =
        build_login_response(&state, access_token, next_refresh_token.token, current_user);

    Ok(Json(ApiResponse::ok(response)))
}

/// GET /api/auth/me
///
/// 注意:从 DB 现查 user 全字段。JWT 里 `full_name` / `email` 没存,
/// 之前的实现直接用 `current_user.full_name`/`email`,永远是 `None`。
pub async fn me(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> AppResult<Json<ApiResponse<UserInfoDto>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());

    let user = auth_repo
        .find_user_by_id(current_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在".to_string()))?;

    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    let response = UserInfoDto {
        user_id: user.user_id,
        username: user.username,
        display_name: user.full_name,
        email: user.email,
        roles,
        permissions,
    };

    Ok(Json(ApiResponse::ok(response)))
}

/// GET /api/auth/roles
///
/// 当前登录用户的有效角色集合。系统角色管理列表在 `/api/system/roles`。
pub async fn roles(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> AppResult<Json<ApiResponse<CurrentUserRolesDto>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());
    let user = auth_repo
        .find_user_by_id(current_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在".to_string()))?;
    let roles = auth_repo.get_user_roles(user.user_id).await?;

    Ok(Json(ApiResponse::ok(CurrentUserRolesDto {
        user_id: user.user_id,
        username: user.username,
        roles,
    })))
}

/// GET /api/auth/permissions
///
/// 当前登录用户的有效权限集合。系统角色权限管理在 `/api/system/roles` 体系下。
pub async fn permissions(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> AppResult<Json<ApiResponse<CurrentUserPermissionsDto>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());
    let user = auth_repo
        .find_user_by_id(current_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在".to_string()))?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    Ok(Json(ApiResponse::ok(CurrentUserPermissionsDto {
        user_id: user.user_id,
        username: user.username,
        permissions,
    })))
}

// ============= helpers =============

fn build_login_response(
    state: &AppState,
    access_token: String,
    refresh_token: String,
    current_user: CurrentUser,
) -> LoginResponse {
    LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt_expires_seconds,
        refresh_expires_in: state.jwt_refresh_expires_seconds,
        user: UserInfoDto {
            user_id: current_user.user_id,
            username: current_user.username,
            display_name: current_user.full_name,
            email: current_user.email,
            roles: current_user.roles,
            permissions: current_user.permissions,
        },
    }
}

fn refresh_token_invalid() -> AppError {
    AppError::Unauthorized("REFRESH_TOKEN_INVALID".to_string())
}

/// 从请求头里提取客户端 IP。优先级:`X-Forwarded-For` 首项 → `X-Real-IP`。
///
/// 部署在反向代理后请确认前置代理设置了这两个头之一,否则始终为 `None`。
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = value.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 防时序攻击的 dummy verify。
///
/// 当请求里的用户名不存在时,直接返 401 会比"用户名存在但密码错"
/// 快一个 Argon2 哈希周期(秒级)的时间,可被外部用作用户名枚举旁道。
/// 此函数生成并验证一次假哈希,把响应时间拉平。
///
/// 假哈希惰性初始化一次(`OnceLock`),后续都是常量代价的 verify 调用。
fn run_dummy_password_verify() {
    use argon2::password_hash::{SaltString, rand_core::OsRng};
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use std::sync::OnceLock;

    static DUMMY_HASH: OnceLock<String> = OnceLock::new();

    let hash_str = DUMMY_HASH.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(b"dummy_password_for_timing_protection", &salt)
            .expect("argon2 hash failed (dummy)")
            .to_string()
    });

    if let Ok(parsed) = PasswordHash::new(hash_str) {
        // 故意忽略结果,只是为了消耗与真实 verify 等量的 CPU。
        let _ = Argon2::default().verify_password(b"any_password", &parsed);
    }
}
