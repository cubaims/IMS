use cuba_shared::{AppError, AppResult};

use crate::{
    application::TraceabilityQueryRepository,
    domain::{
        BatchGenealogyTrace, BatchTraceQuery, BatchTraceReport, SerialTraceQuery, SerialTraceReport,
    },
};

pub struct TraceabilityService<R>
where
    R: TraceabilityQueryRepository,
{
    repository: R,
}

impl<R> TraceabilityService<R>
where
    R: TraceabilityQueryRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn trace_batch(&self, query: BatchTraceQuery) -> AppResult<BatchTraceReport> {
        let options = query.options.normalized();
        let batch_number = query.batch_number.as_str();

        let batch = self
            .repository
            .get_batch_snapshot(batch_number)
            .await?
            .ok_or_else(|| {
                AppError::business(
                    "TRACE_BATCH_NOT_FOUND",
                    format!("批次不存在：{batch_number}"),
                )
            })?;

        let genealogy = if options.include_genealogy {
            Some(BatchGenealogyTrace {
                backward_components: self
                    .repository
                    .list_backward_components(batch_number, options.max_depth)
                    .await?,
                forward_where_used: self
                    .repository
                    .list_forward_where_used(batch_number, options.max_depth)
                    .await?,
            })
        } else {
            None
        };

        let inventory_movements = if options.include_inventory {
            self.repository
                .list_inventory_movements_by_batch(batch_number, options.movement_limit)
                .await?
        } else {
            Vec::new()
        };

        let batch_history = if options.include_history {
            self.repository
                .list_batch_history(batch_number, options.event_limit)
                .await?
        } else {
            Vec::new()
        };

        let (inspection_lots, quality_notifications) = if options.include_quality {
            (
                self.repository
                    .list_inspection_lots_for_batch(batch_number, options.quality_limit)
                    .await?,
                self.repository
                    .list_quality_notifications_for_batch(batch_number, options.quality_limit)
                    .await?,
            )
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(BatchTraceReport {
            batch,
            genealogy,
            inventory_movements,
            batch_history,
            inspection_lots,
            quality_notifications,
        })
    }

    pub async fn trace_serial(&self, query: SerialTraceQuery) -> AppResult<SerialTraceReport> {
        let options = query.options.normalized();
        let serial_number = query.serial_number.as_str();

        let serial = self
            .repository
            .get_serial_snapshot(serial_number)
            .await?
            .ok_or_else(|| {
                AppError::business(
                    "TRACE_SERIAL_NOT_FOUND",
                    format!("序列号不存在：{serial_number}"),
                )
            })?;

        let serial_history = if options.include_history {
            self.repository
                .list_serial_history(serial_number, options.event_limit)
                .await?
        } else {
            Vec::new()
        };

        let inventory_movements = if options.include_inventory {
            self.repository
                .list_inventory_movements_by_serial(serial_number, options.movement_limit)
                .await?
        } else {
            Vec::new()
        };

        let (inspection_lots, quality_notifications) = if options.include_quality {
            (
                self.repository
                    .list_inspection_lots_for_serial(serial_number, options.quality_limit)
                    .await?,
                self.repository
                    .list_quality_notifications_for_serial(serial_number, options.quality_limit)
                    .await?,
            )
        } else {
            (Vec::new(), Vec::new())
        };

        let batch_context = if query.include_batch_context {
            if let Some(batch_number) = serial.batch_number.clone() {
                Some(Box::new(
                    self.trace_batch(BatchTraceQuery {
                        batch_number: crate::domain::BatchNumber::new(batch_number)
                            .map_err(|err| AppError::Validation(err.to_string()))?,
                        options,
                    })
                    .await?,
                ))
            } else {
                None
            }
        } else {
            None
        };

        Ok(SerialTraceReport {
            serial,
            serial_history,
            inventory_movements,
            inspection_lots,
            quality_notifications,
            batch_context,
        })
    }
}
