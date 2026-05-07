use super::dto::{AuthResponse, LoginRequest, LoginResponse, UserInfoDto};
use crate::infrastructure::PostgresAuthRepository;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState, CurrentUser};

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

    let exec_result =
        login_use_case.execute(&user, &req.password, roles.clone(), permissions.clone());

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

    // 4. 成功审计
    let _ = auth_repo
        .write_audit_log(Some(user.user_id), "LOGIN", client_ip)
        .await;

    let response = LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt_expires_seconds,
        user: UserInfoDto {
            user_id: current_user.user_id,
            username: current_user.username,
            display_name: current_user.full_name,
            email: current_user.email,
            roles,
            permissions,
        },
    };

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

pub async fn roles(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse {
        module: "auth",
        status: "roles",
    })))
}

pub async fn permissions(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse {
        module: "auth",
        status: "permissions",
    })))
}

// ============= helpers =============

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
    use argon2::password_hash::{rand_core::OsRng, SaltString};
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
