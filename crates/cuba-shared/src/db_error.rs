use crate::AppError;
use sqlx::Error as SqlxError;

/// 统一的数据库错误映射（库存 + 生产模块）
pub fn map_inventory_db_error(err: SqlxError) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();

    // 优先使用结构化错误码（数据库函数中抛出的自定义 code）
    if let SqlxError::Database(db_err) = &err {
        if let Some(code) = db_err.code() {
            match code.as_ref() {
                "INSUFFICIENT_STOCK" => {
                    return AppError::business("INSUFFICIENT_STOCK", "库存不足，无法完成过账");
                }
                "INSUFFICIENT_BATCH_STOCK" => {
                    return AppError::business(
                        "INSUFFICIENT_BATCH_STOCK",
                        "批次库存不足，无法完成过账",
                    );
                }
                "INSUFFICIENT_BIN_STOCK" => {
                    return AppError::business(
                        "INSUFFICIENT_BIN_STOCK",
                        "货位库存不足，无法完成过账",
                    );
                }
                "BIN_CAPACITY_EXCEEDED" => {
                    return AppError::business(
                        "BIN_CAPACITY_EXCEEDED",
                        "目标货位容量不足，无法入库或转储",
                    );
                }
                "BATCH_FROZEN" => {
                    return AppError::business("BATCH_FROZEN", "批次已冻结，不能出库");
                }
                "BATCH_SCRAPPED" => {
                    return AppError::business("BATCH_SCRAPPED", "批次已报废，不能出库");
                }
                "MATERIAL_NOT_FOUND" => {
                    return AppError::business("MATERIAL_NOT_FOUND", "物料不存在");
                }
                "BIN_NOT_FOUND" => return AppError::business("BIN_NOT_FOUND", "货位不存在"),
                "BATCH_NOT_FOUND" => return AppError::business("BATCH_NOT_FOUND", "批次不存在"),
                _ => {}
            }
        }
    }

    // 兜底字符串匹配（兼容老的数据库函数）
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
    if message.contains("容量") || message.contains("超限") || lower.contains("capacity") {
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

pub fn map_production_db_error(err: SqlxError) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();

    // 生产模块特有错误（先判断）
    if lower.contains("for update cannot be applied to the nullable side") {
        return AppError::business(
            "PRODUCTION_LOCK_ERROR",
            "生产订单锁定失败，请联系管理员检查数据库函数",
        );
    }
    if message.contains("生产订单") && (message.contains("不存在") || lower.contains("not found"))
    {
        return AppError::business("PRODUCTION_ORDER_NOT_FOUND", "生产订单不存在");
    }
    if message.contains("订单状态") || (message.contains("状态") && lower.contains("invalid"))
    {
        return AppError::business(
            "PRODUCTION_ORDER_STATUS_INVALID",
            "生产订单状态不符合操作要求",
        );
    }
    if message.contains("BOM") && (message.contains("不存在") || message.contains("未生效")) {
        return AppError::business("BOM_NOT_FOUND", "BOM 不存在或未生效");
    }
    if message.contains("产品变体") && (message.contains("不存在") || message.contains("未启用"))
    {
        return AppError::business("PRODUCT_VARIANT_NOT_FOUND", "产品变体不存在或未启用");
    }
    if message.contains("工作中心") && (message.contains("不存在") || message.contains("未启用"))
    {
        return AppError::business("WORK_CENTER_NOT_FOUND", "工作中心不存在或未启用");
    }
    if message.contains("组件") && message.contains("库存不足") {
        return AppError::business("COMPONENT_STOCK_SHORTAGE", "组件库存不足，无法完成生产");
    }
    if message.contains("成品批次") && (message.contains("已存在") || lower.contains("duplicate"))
    {
        return AppError::business("FINISHED_BATCH_ALREADY_EXISTS", "成品批次号已存在");
    }
    if message.contains("数量") && (message.contains("超过") || lower.contains("exceed")) {
        return AppError::business("PRODUCTION_QTY_EXCEEDED", "完工数量超过计划剩余数量");
    }

    // 最后 fallback 到库存错误映射
    map_inventory_db_error(err)
}
