use crate::AppError;

pub fn map_inventory_db_error(err: sqlx::Error) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();

    if lower.contains("insufficient stock")
        || message.contains("库存不足")
        || message.contains("负库存")
    {
        return AppError::business("INSUFFICIENT_STOCK", "库存不足，无法完成过账");
    }

    if lower.contains("insufficient batch stock") || message.contains("批次库存不足") {
        return AppError::business("INSUFFICIENT_BATCH_STOCK", "批次库存不足，无法完成过账");
    }

    if lower.contains("insufficient bin stock") || message.contains("货位库存不足") {
        return AppError::business("INSUFFICIENT_BIN_STOCK", "货位库存不足，无法完成过账");
    }

    if lower.contains("capacity") || message.contains("容量") || message.contains("超限") {
        return AppError::business("BIN_CAPACITY_EXCEEDED", "目标货位容量不足，无法入库或转储");
    }

    if message.contains("冻结") || lower.contains("frozen") {
        return AppError::business("BATCH_FROZEN", "批次已冻结，不能出库");
    }

    if message.contains("报废") || lower.contains("scrap") || lower.contains("scrapped") {
        return AppError::business("BATCH_SCRAPPED", "批次已报废，不能出库");
    }

    if message.contains("物料") && message.contains("不存在") {
        return AppError::business("MATERIAL_NOT_FOUND", "物料不存在");
    }

    if message.contains("货位") && message.contains("不存在") {
        return AppError::business("BIN_NOT_FOUND", "货位不存在");
    }

    if message.contains("批次") && message.contains("不存在") {
        return AppError::business("BATCH_NOT_FOUND", "批次不存在");
    }

    AppError::Database(err)
}
