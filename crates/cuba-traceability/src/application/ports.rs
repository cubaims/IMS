use async_trait::async_trait;
use cuba_shared::AppResult;

use crate::domain::{
    BatchGenealogyLink, BatchHistoryTrace, BatchSnapshot, InspectionLotTrace,
    InventoryMovementTrace, QualityNotificationTrace, SerialHistoryTrace, SerialSnapshot,
};

#[async_trait]
pub trait TraceabilityQueryRepository: Send + Sync {
    async fn get_batch_snapshot(&self, batch_number: &str) -> AppResult<Option<BatchSnapshot>>;

    async fn get_serial_snapshot(&self, serial_number: &str) -> AppResult<Option<SerialSnapshot>>;

    async fn list_backward_components(
        &self,
        batch_number: &str,
        max_depth: u32,
    ) -> AppResult<Vec<BatchGenealogyLink>>;

    async fn list_forward_where_used(
        &self,
        batch_number: &str,
        max_depth: u32,
    ) -> AppResult<Vec<BatchGenealogyLink>>;

    async fn list_inventory_movements_by_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InventoryMovementTrace>>;

    async fn list_inventory_movements_by_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InventoryMovementTrace>>;

    async fn list_batch_history(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<BatchHistoryTrace>>;

    async fn list_serial_history(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<SerialHistoryTrace>>;

    async fn list_inspection_lots_for_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InspectionLotTrace>>;

    async fn list_inspection_lots_for_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InspectionLotTrace>>;

    async fn list_quality_notifications_for_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<QualityNotificationTrace>>;

    async fn list_quality_notifications_for_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<QualityNotificationTrace>>;
}
