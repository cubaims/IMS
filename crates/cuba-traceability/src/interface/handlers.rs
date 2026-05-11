use axum::{
    Json,
    extract::{Path, Query, State},
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState};

use crate::{
    application::TraceabilityService,
    domain::{BatchNumber, BatchTraceQuery, SerialNumber, SerialTraceQuery},
    infrastructure::PostgresTraceabilityRepository,
};

use super::dto::{SerialTraceQueryParams, TraceQueryParams, TraceabilityResponse};

fn service(state: &AppState) -> TraceabilityService<PostgresTraceabilityRepository> {
    TraceabilityService::new(PostgresTraceabilityRepository::new(state.db_pool.clone()))
}

pub async fn health(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<TraceabilityResponse>>> {
    Ok(Json(ApiResponse::ok(TraceabilityResponse {
        module: "traceability",
        status: "ready",
    })))
}

pub async fn trace_batch(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Query(query): Query<TraceQueryParams>,
) -> AppResult<Json<ApiResponse<crate::domain::BatchTraceReport>>> {
    let batch_number =
        BatchNumber::new(batch_number).map_err(|err| AppError::Validation(err.to_string()))?;

    let result = service(&state)
        .trace_batch(BatchTraceQuery {
            batch_number,
            options: query.options(),
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn trace_serial(
    State(state): State<AppState>,
    Path(serial_number): Path<String>,
    Query(query): Query<SerialTraceQueryParams>,
) -> AppResult<Json<ApiResponse<crate::domain::SerialTraceReport>>> {
    let include_batch_context = query.include_batch_context();
    let serial_number =
        SerialNumber::new(serial_number).map_err(|err| AppError::Validation(err.to_string()))?;

    let result = service(&state)
        .trace_serial(SerialTraceQuery {
            serial_number,
            include_batch_context,
            options: query.options(),
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}
