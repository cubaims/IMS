use cuba_shared::AppError;

pub fn map_db_error_text(message: &str) -> AppError {
    let lower = message.to_lowercase();

    if message.contains("生产订单不存在") || lower.contains("production order") && lower.contains("not found") {
        return AppError::NotFound("PRODUCTION_ORDER_NOT_FOUND".to_string());
    }

    if message.contains("只有已下达") || message.contains("状态") || lower.contains("status") {
        return AppError::Validation("PRODUCTION_ORDER_STATUS_INVALID".to_string());
    }

    if message.contains("完工数量") || lower.contains("quantity") {
        return AppError::Validation("PRODUCTION_QTY_INVALID".to_string());
    }

    if message.contains("超过计划") || lower.contains("exceed") {
        return AppError::Validation("PRODUCTION_QTY_EXCEEDED".to_string());
    }

    if message.contains("BOM") && (message.contains("不存在") || lower.contains("not found")) {
        return AppError::NotFound("BOM_NOT_FOUND".to_string());
    }

    if message.contains("BOM") && (message.contains("未生效") || message.contains("未启用")) {
        return AppError::Validation("BOM_INACTIVE".to_string());
    }

    if message.contains("组件") && message.contains("库存不足") {
        return AppError::Validation("COMPONENT_STOCK_SHORTAGE".to_string());
    }

    if message.contains("库存不足") || lower.contains("insufficient stock") {
        return AppError::Validation("INSUFFICIENT_STOCK".to_string());
    }

    if message.contains("批次") && message.contains("不存在") {
        return AppError::NotFound("COMPONENT_BATCH_NOT_FOUND".to_string());
    }

    if message.contains("冻结") {
        return AppError::Validation("BATCH_FROZEN".to_string());
    }

    if message.contains("报废") {
        return AppError::Validation("BATCH_SCRAPPED".to_string());
    }

    if message.contains("成品批次") && message.contains("存在") {
        return AppError::Validation("FINISHED_BATCH_ALREADY_EXISTS".to_string());
    }

    if message.contains("货位") && message.contains("容量") {
        return AppError::Validation("FINISHED_BIN_CAPACITY_EXCEEDED".to_string());
    }

    if message.contains("genealogy") || message.contains("谱系") {
        return AppError::Internal("GENEALOGY_WRITE_FAILED".to_string());
    }

    if message.contains("variance") || message.contains("差异") {
        return AppError::Internal("PRODUCTION_VARIANCE_WRITE_FAILED".to_string());
    }

    AppError::Internal(message.to_string())
}