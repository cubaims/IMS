use super::dto::{AuthResponse, LoginRequest, LoginResponse, UserInfoDto};
use crate::infrastructure::PostgresAuthRepository;
use axum::{
    Json,
    extract::{Extension, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState, CurrentUser};

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
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());

    let user = auth_repo
        .find_user_by_username(&req.username)
        .await?
        .ok_or_else(|| cuba_shared::AppError::Unauthorized("用户名或密码错误".to_string()))?;

    let roles = auth_repo.get_user_roles(user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(user.user_id).await?;

    let login_use_case = crate::application::LoginUseCase::new(
        state.jwt_secret.clone(),
        state.jwt_issuer.clone(),
        state.jwt_expires_seconds,
    );

    let (token, current_user) =
        login_use_case.execute(&user, &req.password, roles.clone(), permissions.clone())?;

    // 记录审计日志
    let _ = auth_repo
        .write_audit_log(Some(user.user_id), "LOGIN", None)
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

pub async fn me(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> AppResult<Json<ApiResponse<UserInfoDto>>> {
    let auth_repo = PostgresAuthRepository::new(state.db_pool.clone());

    let roles = auth_repo.get_user_roles(current_user.user_id).await?;
    let permissions = auth_repo.get_user_permissions(current_user.user_id).await?;

    let response = UserInfoDto {
        user_id: current_user.user_id,
        username: current_user.username,
        display_name: current_user.full_name,
        email: current_user.email,
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
