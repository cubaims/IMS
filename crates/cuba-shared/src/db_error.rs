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

pub fn map_production_db_error(err: sqlx::Error) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();

    // FOR UPDATE with LEFT JOIN error
    if lower.contains("for update cannot be applied to the nullable side") {
        return AppError::business(
            "PRODUCTION_LOCK_ERROR",
            "生产订单锁定失败，请联系管理员检查数据库函数",
        );
    }

    // Production order not found
    if message.contains("生产订单") && (message.contains("不存在") || lower.contains("not found")) {
        return AppError::business("PRODUCTION_ORDER_NOT_FOUND", "生产订单不存在");
    }

    // Production order status invalid
    if message.contains("订单状态") || (message.contains("状态") && lower.contains("invalid")) {
        return AppError::business("PRODUCTION_ORDER_STATUS_INVALID", "生产订单状态不符合操作要求");
    }

    // BOM not found or inactive
    if message.contains("BOM") && (message.contains("不存在") || message.contains("未生效")) {
        return AppError::business("BOM_NOT_FOUND", "BOM 不存在或未生效");
    }

    // Product variant not found or inactive
    if message.contains("产品变体") && (message.contains("不存在") || message.contains("未启用")) {
        return AppError::business("PRODUCT_VARIANT_NOT_FOUND", "产品变体不存在或未启用");
    }

    // Work center not found or inactive
    if message.contains("工作中心") && (message.contains("不存在") || message.contains("未启用")) {
        return AppError::business("WORK_CENTER_NOT_FOUND", "工作中心不存在或未启用");
    }

    // Component stock shortage
    if message.contains("组件") && message.contains("库存不足") {
        return AppError::business("COMPONENT_STOCK_SHORTAGE", "组件库存不足，无法完成生产");
    }

    // Finished batch already exists
    if message.contains("成品批次") && (message.contains("已存在") || lower.contains("duplicate")) {
        return AppError::business("FINISHED_BATCH_ALREADY_EXISTS", "成品批次号已存在");
    }

    // Target bin not found
    if message.contains("目标货位") && message.contains("未找到") {
        return AppError::business("TARGET_BIN_NOT_FOUND", "未找到成品目标货位");
    }

    // Quantity exceeded
    if message.contains("数量") && (message.contains("超过") || lower.contains("exceed")) {
        return AppError::business("PRODUCTION_QTY_EXCEEDED", "完工数量超过计划剩余数量");
    }

    // Variance write failed
    if message.contains("差异") || lower.contains("variance") {
        return AppError::business("PRODUCTION_VARIANCE_WRITE_FAILED", "生产成本差异记录失败");
    }

    // Genealogy write failed
    if message.contains("谱系") || lower.contains("genealogy") {
        return AppError::business("BATCH_GENEALOGY_WRITE_FAILED", "批次谱系记录失败");
    }

    // No component lines
    if message.contains("组件行") || (message.contains("component") && lower.contains("line")) {
        return AppError::business("NO_COMPONENT_LINES", "生产订单没有组件行");
    }

    // Fallback to inventory error mapping for stock-related issues
    if lower.contains("stock") || message.contains("库存") {
        return map_inventory_db_error(err);
    }

    AppError::Database(err)
}
