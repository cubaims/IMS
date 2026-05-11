//! cuba-auth: 用户认证 / 授权
//!
//! 模块布局遵循 hexagonal:
//! - domain: 数据载体(JwtClaims, User, Role)
//! - application: 业务用例(login, authorize, get current user) + JWT 验签
//! - infrastructure: PG 访问层
//! - interface: HTTP DTO / handlers / routes

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

// 显式 re-export,避免 `pub use ::*` 的撞名风险。
pub use application::{
    ACCESS_TOKEN_INVALIDATION_POLICY, AccessTokenInvalidationPolicy, AuthorizeUseCase,
    DEFAULT_ACCESS_TOKEN_EXPIRES_SECONDS, GetCurrentUserUseCase, IssuedRefreshToken, LoginUseCase,
    ParsedRefreshToken, VerifyError, current_user_from_fresh_grants, ensure_refresh_token_usable,
    ensure_refresh_user_enabled, issue_refresh_token, parse_refresh_token, verify_access_token,
    verify_refresh_secret,
};
pub use domain::{JwtClaims, Role, User};
pub use infrastructure::PostgresAuthRepository;
pub use interface::dto::{
    AuthResponse, CurrentUserPermissionsDto, CurrentUserRolesDto, LoginRequest, LoginResponse,
    RefreshTokenRequest, UserInfoDto,
};
pub use interface::{handlers, routes};
