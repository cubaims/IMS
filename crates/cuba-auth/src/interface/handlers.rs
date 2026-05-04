use super::dto::{AuthResponse, LoginRequest, LoginResponse, UserInfoDto};
use crate::application::LoginUseCase;
use crate::infrastructure::PostgresAuthRepository;
use axum::{Json, extract::State};
use cuba_shared::{ApiResponse, AppResult, AppState};

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse {
        module: "auth",
        status: "ready",
    })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    // 从状态中获取必要的组件
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());

    // 查找用户
    let user = auth_repo
        .find_user_by_username(&req.username)
        .await?
        .ok_or_else(|| cuba_shared::AppError::Unauthorized("用户名或密码错误".to_string()))?;

    // 获取用户角色和权限
    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    // 创建登录用例并执行
    let login_use_case = LoginUseCase::new(
        state.jwt_secret.clone(),
        "cuba-ims".to_string(),
        86400, // 24小时
    );

    let (token, current_user) =
        login_use_case.execute(&user, &req.password, roles.clone(), permissions.clone())?;

    // 记录审计日志
    auth_repo
        .write_audit_log(Some(user.user_id), "LOGIN", None)
        .await?;

    // 构造响应
    let response = LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
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

pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<Json<ApiResponse<UserInfoDto>>> {
    use crate::domain::JwtClaims;
    use jsonwebtoken::{DecodingKey, Validation, decode};

    // 提取 Token
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            cuba_shared::AppError::Unauthorized("缺少 Authorization header".to_string())
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err(cuba_shared::AppError::Unauthorized(
            "Authorization 格式错误".to_string(),
        ));
    }
    let token = &auth_header[7..];

    // 解析 JWT
    let claims = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| cuba_shared::AppError::Internal(e.to_string()))?
    .claims;

    // 查询用户最新状态
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());
    let user = auth_repo
        .find_user_by_username(&claims.username)
        .await?
        .ok_or_else(|| cuba_shared::AppError::Unauthorized("用户不存在".to_string()))?;

    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    let response = UserInfoDto {
        user_id: claims.sub,
        username: claims.username,
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
