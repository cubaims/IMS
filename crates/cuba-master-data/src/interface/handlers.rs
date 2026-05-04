use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::MasterDataResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "ready" })))
}

pub async fn materials(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "materials" })))
}


pub async fn bins(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "bins" })))
}


pub async fn suppliers(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "suppliers" })))
}


pub async fn customers(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "customers" })))
}


pub async fn boms(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "boms" })))
}


pub async fn variants(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MasterDataResponse>>> {
    Ok(Json(ApiResponse::ok(MasterDataResponse { module: "master_data", status: "variants" })))
}

