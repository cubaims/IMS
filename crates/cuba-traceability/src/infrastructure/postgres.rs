use async_trait::async_trait;
use cuba_shared::{AppError, AppResult};
use sqlx::{PgPool, Row};

use crate::{
    application::TraceabilityQueryRepository,
    domain::{
        BatchGenealogyLink, BatchHistoryTrace, BatchSnapshot, InspectionLotTrace,
        InventoryMovementTrace, QualityNotificationTrace, SerialHistoryTrace, SerialSnapshot,
    },
};

#[derive(Clone)]
pub struct PostgresTraceabilityRepository {
    pool: PgPool,
}

impl PostgresTraceabilityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_traceability_db_error(context: &'static str, error: sqlx::Error) -> AppError {
        match cuba_shared::map_traceability_db_error(error) {
            AppError::Business {
                code: "TRACE_QUERY_FAILED",
                ..
            } => AppError::business("TRACE_QUERY_FAILED", format!("{context}失败")),
            mapped => mapped,
        }
    }
}

#[async_trait]
impl TraceabilityQueryRepository for PostgresTraceabilityRepository {
    async fn get_batch_snapshot(&self, batch_number: &str) -> AppResult<Option<BatchSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT
                b.batch_number,
                b.material_id,
                m.material_name,
                b.production_date,
                b.expiry_date,
                b.supplier_batch,
                b.quality_grade,
                b.current_stock::NUMERIC AS current_stock,
                b.current_bin,
                b.quality_status::TEXT AS quality_status,
                COALESCE(b.created_at, NOW()) AS created_at,
                COALESCE(b.updated_at, NOW()) AS updated_at
            FROM wms.wms_batches b
            JOIN mdm.mdm_materials m ON m.material_id = b.material_id
            WHERE b.batch_number = $1
            "#,
        )
        .bind(batch_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询批次快照", err))?;

        Ok(row.map(|row| BatchSnapshot {
            batch_number: row.get("batch_number"),
            material_id: row.get("material_id"),
            material_name: row.get("material_name"),
            production_date: row.get("production_date"),
            expiry_date: row.get("expiry_date"),
            supplier_batch: row.get("supplier_batch"),
            quality_grade: row.get("quality_grade"),
            current_stock: row.get("current_stock"),
            current_bin: row.get("current_bin"),
            quality_status: row.get("quality_status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    async fn get_serial_snapshot(&self, serial_number: &str) -> AppResult<Option<SerialSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT
                s.serial_number,
                s.material_id,
                m.material_name,
                s.batch_number,
                s.current_status,
                s.current_bin,
                s.quality_status::TEXT AS quality_status,
                s.last_movement_at,
                COALESCE(s.created_at, NOW()) AS created_at,
                COALESCE(s.updated_at, NOW()) AS updated_at
            FROM wms.wms_serial_numbers s
            JOIN mdm.mdm_materials m ON m.material_id = s.material_id
            WHERE s.serial_number = $1
            "#,
        )
        .bind(serial_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询序列号快照", err))?;

        Ok(row.map(|row| SerialSnapshot {
            serial_number: row.get("serial_number"),
            material_id: row.get("material_id"),
            material_name: row.get("material_name"),
            batch_number: row.get("batch_number"),
            current_status: row.get("current_status"),
            current_bin: row.get("current_bin"),
            quality_status: row.get("quality_status"),
            last_movement_at: row.get("last_movement_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    async fn list_backward_components(
        &self,
        batch_number: &str,
        max_depth: u32,
    ) -> AppResult<Vec<BatchGenealogyLink>> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE backward_trace AS (
                SELECT
                    1::INT AS level,
                    g.parent_batch_number,
                    g.component_batch_number,
                    g.parent_material_id,
                    g.component_material_id,
                    g.production_order_id,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id
                FROM wms.wms_batch_genealogy g
                WHERE g.parent_batch_number = $1

                UNION ALL

                SELECT
                    bt.level + 1,
                    g.parent_batch_number,
                    g.component_batch_number,
                    g.parent_material_id,
                    g.component_material_id,
                    g.production_order_id,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id
                FROM wms.wms_batch_genealogy g
                JOIN backward_trace bt ON g.parent_batch_number = bt.component_batch_number
                WHERE bt.level < $2
            )
            SELECT *
            FROM backward_trace
            ORDER BY level, component_batch_number, parent_batch_number
            "#,
        )
        .bind(batch_number)
        .bind(i32::try_from(max_depth).unwrap_or(20))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询批次反向谱系", err))?;

        Ok(rows
            .into_iter()
            .map(|row| BatchGenealogyLink {
                level: row.get("level"),
                parent_batch_number: row.get("parent_batch_number"),
                component_batch_number: row.get("component_batch_number"),
                parent_material_id: row.get("parent_material_id"),
                component_material_id: row.get("component_material_id"),
                production_order_id: row.get("production_order_id"),
                consumed_qty: row.get("consumed_qty"),
                output_qty: row.get("output_qty"),
                transaction_id: row.get("transaction_id"),
            })
            .collect())
    }

    async fn list_forward_where_used(
        &self,
        batch_number: &str,
        max_depth: u32,
    ) -> AppResult<Vec<BatchGenealogyLink>> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE forward_trace AS (
                SELECT
                    1::INT AS level,
                    g.parent_batch_number,
                    g.component_batch_number,
                    g.parent_material_id,
                    g.component_material_id,
                    g.production_order_id,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id
                FROM wms.wms_batch_genealogy g
                WHERE g.component_batch_number = $1

                UNION ALL

                SELECT
                    ft.level + 1,
                    g.parent_batch_number,
                    g.component_batch_number,
                    g.parent_material_id,
                    g.component_material_id,
                    g.production_order_id,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id
                FROM wms.wms_batch_genealogy g
                JOIN forward_trace ft ON g.component_batch_number = ft.parent_batch_number
                WHERE ft.level < $2
            )
            SELECT *
            FROM forward_trace
            ORDER BY level, parent_batch_number, component_batch_number
            "#,
        )
        .bind(batch_number)
        .bind(i32::try_from(max_depth).unwrap_or(20))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询批次正向谱系", err))?;

        Ok(rows
            .into_iter()
            .map(|row| BatchGenealogyLink {
                level: row.get("level"),
                parent_batch_number: row.get("parent_batch_number"),
                component_batch_number: row.get("component_batch_number"),
                parent_material_id: row.get("parent_material_id"),
                component_material_id: row.get("component_material_id"),
                production_order_id: row.get("production_order_id"),
                consumed_qty: row.get("consumed_qty"),
                output_qty: row.get("output_qty"),
                transaction_id: row.get("transaction_id"),
            })
            .collect())
    }

    async fn list_inventory_movements_by_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InventoryMovementTrace>> {
        list_inventory_movements(&self.pool, "batch_number", batch_number, limit).await
    }

    async fn list_inventory_movements_by_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InventoryMovementTrace>> {
        list_inventory_movements(&self.pool, "serial_number", serial_number, limit).await
    }

    async fn list_batch_history(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<BatchHistoryTrace>> {
        let rows = sqlx::query(
            r#"
            SELECT
                h.id AS history_id,
                h.batch_number,
                b.material_id,
                COALESCE(h.change_reason, 'BATCH_CHANGE') AS event_type,
                h.old_quality_status::TEXT AS old_quality_status,
                h.new_quality_status::TEXT AS new_quality_status,
                h.old_bin,
                h.new_bin,
                h.old_stock::NUMERIC AS old_stock,
                h.new_stock::NUMERIC AS new_stock,
                h.qty_change::NUMERIC AS qty_change,
                h.transaction_id,
                h.inspection_lot_id,
                h.notification_id,
                h.changed_by,
                COALESCE(h.changed_at, NOW()) AS changed_at,
                h.change_reason AS remarks
            FROM wms.wms_batch_history h
            JOIN wms.wms_batches b ON b.batch_number = h.batch_number
            WHERE h.batch_number = $1
            ORDER BY h.changed_at DESC, h.id DESC
            LIMIT $2
            "#,
        )
        .bind(batch_number)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询批次历史", err))?;

        Ok(rows
            .into_iter()
            .map(|row| BatchHistoryTrace {
                history_id: row.get("history_id"),
                batch_number: row.get("batch_number"),
                material_id: row.get("material_id"),
                event_type: row.get("event_type"),
                old_quality_status: row.get("old_quality_status"),
                new_quality_status: row.get("new_quality_status"),
                old_bin: row.get("old_bin"),
                new_bin: row.get("new_bin"),
                old_stock: row.get("old_stock"),
                new_stock: row.get("new_stock"),
                qty_change: row.get("qty_change"),
                transaction_id: row.get("transaction_id"),
                inspection_lot_id: row.get("inspection_lot_id"),
                notification_id: row.get("notification_id"),
                changed_by: row.get("changed_by"),
                changed_at: row.get("changed_at"),
                remarks: row.get("remarks"),
            })
            .collect())
    }

    async fn list_serial_history(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<SerialHistoryTrace>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id AS history_id,
                serial_number,
                old_status,
                new_status,
                old_bin,
                new_bin,
                old_quality_status::TEXT AS old_quality_status,
                new_quality_status::TEXT AS new_quality_status,
                transaction_id,
                changed_by,
                COALESCE(changed_at, NOW()) AS changed_at
            FROM wms.wms_serial_history
            WHERE serial_number = $1
            ORDER BY changed_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(serial_number)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| Self::map_traceability_db_error("查询序列号历史", err))?;

        Ok(rows
            .into_iter()
            .map(|row| SerialHistoryTrace {
                history_id: row.get("history_id"),
                serial_number: row.get("serial_number"),
                old_status: row.get("old_status"),
                new_status: row.get("new_status"),
                old_bin: row.get("old_bin"),
                new_bin: row.get("new_bin"),
                old_quality_status: row.get("old_quality_status"),
                new_quality_status: row.get("new_quality_status"),
                transaction_id: row.get("transaction_id"),
                changed_by: row.get("changed_by"),
                changed_at: row.get("changed_at"),
            })
            .collect())
    }

    async fn list_inspection_lots_for_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InspectionLotTrace>> {
        list_inspection_lots(&self.pool, "batch_number", batch_number, limit).await
    }

    async fn list_inspection_lots_for_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<InspectionLotTrace>> {
        list_inspection_lots(&self.pool, "serial_number", serial_number, limit).await
    }

    async fn list_quality_notifications_for_batch(
        &self,
        batch_number: &str,
        limit: u32,
    ) -> AppResult<Vec<QualityNotificationTrace>> {
        list_quality_notifications(&self.pool, "batch_number", batch_number, limit).await
    }

    async fn list_quality_notifications_for_serial(
        &self,
        serial_number: &str,
        limit: u32,
    ) -> AppResult<Vec<QualityNotificationTrace>> {
        list_quality_notifications(&self.pool, "serial_number", serial_number, limit).await
    }
}

async fn list_inventory_movements(
    pool: &PgPool,
    target_column: &'static str,
    target_value: &str,
    limit: u32,
) -> AppResult<Vec<InventoryMovementTrace>> {
    let sql = format!(
        r#"
        SELECT
            transaction_id,
            transaction_date,
            movement_type::TEXT AS movement_type,
            material_id,
            quantity::NUMERIC AS quantity,
            from_bin,
            to_bin,
            batch_number,
            serial_number,
            reference_doc,
            operator,
            quality_status::TEXT AS quality_status,
            notes
        FROM wms.wms_transactions
        WHERE {target_column} = $1
        ORDER BY transaction_date DESC, id DESC
        LIMIT $2
        "#
    );

    let rows = sqlx::query(&sql)
        .bind(target_value)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(|err| {
            PostgresTraceabilityRepository::map_traceability_db_error("查询库存流转历史", err)
        })?;

    Ok(rows
        .into_iter()
        .map(|row| InventoryMovementTrace {
            transaction_id: row.get("transaction_id"),
            transaction_date: row.get("transaction_date"),
            movement_type: row.get("movement_type"),
            material_id: row.get("material_id"),
            quantity: row.get("quantity"),
            from_bin: row.get("from_bin"),
            to_bin: row.get("to_bin"),
            batch_number: row.get("batch_number"),
            serial_number: row.get("serial_number"),
            reference_doc: row.get("reference_doc"),
            operator: row.get("operator"),
            quality_status: row.get("quality_status"),
            notes: row.get("notes"),
        })
        .collect())
}

async fn list_inspection_lots(
    pool: &PgPool,
    target_column: &'static str,
    target_value: &str,
    limit: u32,
) -> AppResult<Vec<InspectionLotTrace>> {
    let sql = format!(
        r#"
        SELECT
            inspection_lot_id,
            material_id,
            batch_number,
            serial_number,
            inspection_type,
            lot_status::TEXT AS lot_status,
            inspection_date,
            inspector,
            inspection_result,
            COALESCE(created_at, NOW()) AS created_at,
            COALESCE(updated_at, NOW()) AS updated_at
        FROM wms.wms_inspection_lots
        WHERE {target_column} = $1
        ORDER BY COALESCE(inspection_date, created_at) DESC, inspection_lot_id DESC
        LIMIT $2
        "#
    );

    let rows = sqlx::query(&sql)
        .bind(target_value)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(|err| {
            PostgresTraceabilityRepository::map_traceability_db_error("查询质量检验批", err)
        })?;

    Ok(rows
        .into_iter()
        .map(|row| InspectionLotTrace {
            inspection_lot_id: row.get("inspection_lot_id"),
            material_id: row.get("material_id"),
            batch_number: row.get("batch_number"),
            serial_number: row.get("serial_number"),
            inspection_type: row.get("inspection_type"),
            lot_status: row.get("lot_status"),
            inspection_date: row.get("inspection_date"),
            inspector: row.get("inspector"),
            inspection_result: row.get("inspection_result"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn list_quality_notifications(
    pool: &PgPool,
    target_column: &'static str,
    target_value: &str,
    limit: u32,
) -> AppResult<Vec<QualityNotificationTrace>> {
    let sql = format!(
        r#"
        SELECT
            notification_id,
            inspection_lot_id,
            material_id,
            batch_number,
            serial_number,
            defect_code,
            problem_description,
            severity,
            root_cause,
            corrective_action,
            responsible_person,
            status,
            COALESCE(created_at, NOW()) AS created_at,
            closed_at
        FROM wms.wms_quality_notifications
        WHERE {target_column} = $1
        ORDER BY created_at DESC, notification_id DESC
        LIMIT $2
        "#
    );

    let rows = sqlx::query(&sql)
        .bind(target_value)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(|err| {
            PostgresTraceabilityRepository::map_traceability_db_error("查询质量通知", err)
        })?;

    Ok(rows
        .into_iter()
        .map(|row| QualityNotificationTrace {
            notification_id: row.get("notification_id"),
            inspection_lot_id: row.get("inspection_lot_id"),
            material_id: row.get("material_id"),
            batch_number: row.get("batch_number"),
            serial_number: row.get("serial_number"),
            defect_code: row.get("defect_code"),
            problem_description: row.get("problem_description"),
            severity: row.get("severity"),
            root_cause: row.get("root_cause"),
            corrective_action: row.get("corrective_action"),
            responsible_person: row.get("responsible_person"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            closed_at: row.get("closed_at"),
        })
        .collect())
}
