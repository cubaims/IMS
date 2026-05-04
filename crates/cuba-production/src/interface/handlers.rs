use super::dto::ProductionResponse;
use axum::{Json, extract::State};
use cuba_shared::{ApiResponse, AppResult, AppState};

pub async fn health(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProductionResponse>>> {
    Ok(Json(ApiResponse::ok(ProductionResponse {
        module: "production",
        status: "ready",
    })))
}

pub async fn production_orders(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProductionResponse>>> {
    Ok(Json(ApiResponse::ok(ProductionResponse {
        module: "production",
        status: "production-orders",
    })))
}

pub async fn complete(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProductionResponse>>> {
    Ok(Json(ApiResponse::ok(ProductionResponse {
        module: "production",
        status: "complete",
    })))
}

pub async fn variance(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProductionResponse>>> {
    Ok(Json(ApiResponse::ok(ProductionResponse {
        module: "production",
        status: "variance",
    })))
}

pub async fn bom_explosion(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ProductionResponse>>> {
    Ok(Json(ApiResponse::ok(ProductionResponse {
        module: "production",
        status: "bom-explosion",
    })))
}
