//! 数据库错误到 `AppError` 的统一映射。
//!
//! 优先级:
//! 1. SQLSTATE 结构化错误码(数据库函数 `RAISE ... USING ERRCODE = '...'`)
//! 2. 中文关键词兜底(应对未带 ERRCODE 的旧函数)
//!
//! 设计变更:删除了 `lower.contains(...)` 的英文关键词兜底——sqlx 自身的英文错误
//! 文本里 `capacity` / `frozen` / `scrap` 等词出现概率不低,会误判。中长期应让
//! 所有数据库函数都通过 `USING ERRCODE` 抛 SQLSTATE,然后第二段兜底也可去掉。

use crate::AppError;
use sqlx::Error as SqlxError;

/// 主数据(master-data)模块的 sqlx 错误映射。
///
/// 主数据 CRUD 主要触发的是 PostgreSQL 约束类错误,而不是库存类的业务函数 RAISE。
/// 我们按 SQLSTATE 把常见约束转成对前端友好的 `Validation`(可在 UI 上提示用户修正),
/// 其余的(连接错、解码错等)走通用 `Database`,进 INTERNAL_SERVER_ERROR 并落日志。
pub fn map_master_data_db_error(err: SqlxError) -> AppError {
    if let SqlxError::Database(db_err) = &err {
        if let Some(code) = db_err.code() {
            // 真实约束细节(包含表名、约束名)落日志,不进响应
            match code.as_ref() {
                // unique_violation
                "23505" => {
                    tracing::warn!(error = %db_err.message(), "unique constraint violated");
                    return AppError::Validation(
                        "记录已存在:违反唯一约束(主键或唯一字段重复)".to_string(),
                    );
                }
                // foreign_key_violation
                "23503" => {
                    tracing::warn!(error = %db_err.message(), "fk constraint violated");
                    return AppError::Validation(
                        "外键约束失败:引用的关联记录不存在,或本记录正被其他数据引用而无法删除"
                            .to_string(),
                    );
                }
                // not_null_violation
                "23502" => {
                    tracing::warn!(error = %db_err.message(), "not null constraint violated");
                    return AppError::Validation("必填字段缺失".to_string());
                }
                // check_violation
                "23514" => {
                    tracing::warn!(error = %db_err.message(), "check constraint violated");
                    return AppError::Validation(
                        "数据值不符合约束规则,请检查取值范围或格式".to_string(),
                    );
                }
                _ => {}
            }
        }
    }
    AppError::raw_database(err)
}

pub fn map_inventory_db_error(err: SqlxError) -> AppError {
    // ===== 1. SQLSTATE 结构化错误码 =====
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

    // ===== 2. 中文关键词兜底(英文匹配已删除以避免误命中) =====
    let message = err.to_string();
    if message.contains("库存不足") || message.contains("负库存") {
        return AppError::business("INSUFFICIENT_STOCK", "库存不足，无法完成过账");
    }
    if message.contains("批次库存不足") {
        return AppError::business("INSUFFICIENT_BATCH_STOCK", "批次库存不足，无法完成过账");
    }
    if message.contains("货位库存不足") {
        return AppError::business("INSUFFICIENT_BIN_STOCK", "货位库存不足，无法完成过账");
    }
    if message.contains("容量") || message.contains("超限") {
        return AppError::business("BIN_CAPACITY_EXCEEDED", "目标货位容量不足，无法入库或转储");
    }
    if message.contains("冻结") {
        return AppError::business("BATCH_FROZEN", "批次已冻结，不能出库");
    }
    if message.contains("报废") {
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

    AppError::raw_database(err)
}

pub fn map_production_db_error(err: SqlxError) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();

    // 这一条来自 PostgreSQL 自身,短语很具体不会误判,保留英文匹配
    if lower.contains("for update cannot be applied to the nullable side") {
        return AppError::business(
            "PRODUCTION_LOCK_ERROR",
            "生产订单锁定失败，请联系管理员检查数据库函数",
        );
    }

    // 中文关键词兜底
    if message.contains("生产订单") && message.contains("不存在") {
        return AppError::business("PRODUCTION_ORDER_NOT_FOUND", "生产订单不存在");
    }
    if message.contains("订单状态") {
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
    if message.contains("成品批次") && message.contains("已存在") {
        return AppError::business("FINISHED_BATCH_ALREADY_EXISTS", "成品批次号已存在");
    }
    if message.contains("数量") && message.contains("超过") {
        return AppError::business("PRODUCTION_QTY_EXCEEDED", "完工数量超过计划剩余数量");
    }

    map_inventory_db_error(err)
}
