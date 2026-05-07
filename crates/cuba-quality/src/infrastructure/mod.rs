use serde_json::Value;
use sqlx::PgPool;

use crate::application::{BatchHistoryQuery, InspectionLotQuery, QualityApplicationError};
use crate::domain::InspectionLotId;

#[derive(Debug, Clone)]
pub struct PostgresQualityStore {
    pool: PgPool,
}

impl PostgresQualityStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, _query: InspectionLotQuery) -> Result<Value, QualityApplicationError> {
        let _ = &self.pool;
        Ok(serde_json::json!({
            "items": [],
            "message": "inspection lot list not implemented yet"
        }))
    }

    pub async fn find_by_id(
        &self,
        lot_id: &InspectionLotId,
    ) -> Result<Value, QualityApplicationError> {
        let _ = &self.pool;
        Ok(serde_json::json!({
            "inspection_lot_id": lot_id.as_str(),
            "message": "inspection lot detail not implemented yet"
        }))
    }

    pub async fn find_by_lot_id(
        &self,
        lot_id: &InspectionLotId,
    ) -> Result<Value, QualityApplicationError> {
        let _ = &self.pool;
        Ok(serde_json::json!({
            "inspection_lot_id": lot_id.as_str(),
            "results": [],
            "message": "inspection results not implemented yet"
        }))
    }

    pub async fn list_batch_history(
        &self,
        batch_number: &str,
        _query: BatchHistoryQuery,
    ) -> Result<Value, QualityApplicationError> {
        let _ = &self.pool;
        Ok(serde_json::json!({
            "batch_number": batch_number,
            "items": [],
            "message": "batch quality history not implemented yet"
        }))
    }
}

#[derive(Debug, Clone, Default)]
pub struct PostgresQualityIdGenerator;
