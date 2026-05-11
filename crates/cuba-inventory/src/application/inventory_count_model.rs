use rust_decimal::Decimal;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::application::common::PageQuery;
use crate::domain::{InventoryCountScope, InventoryCountStatus, InventoryCountType};

/// 创建盘点单输入
pub struct CreateInventoryCountInput {
    pub count_type: InventoryCountType,
    pub count_scope: InventoryCountScope,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    /// 当前登录用户
    pub operator: String,

    pub remark: Option<String>,
}

/// 生成盘点明细输入
pub struct GenerateInventoryCountLinesInput {
    pub count_doc_id: String,
    pub operator: String,
}

/// 查询盘点单列表输入
///
/// 不做读写分离，所以这里不叫 Query。
/// 只是 service/repository 的普通入参。
#[derive(Debug, Clone, Deserialize)]
pub struct ListInventoryCountsInput {
    pub status: Option<InventoryCountStatus>,
    pub count_type: Option<InventoryCountType>,
    pub count_scope: Option<InventoryCountScope>,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub created_by: Option<String>,

    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,

    #[serde(flatten)]
    pub page: PageQuery,
}

/// 查询盘点单详情输入
pub struct GetInventoryCountInput {
    pub count_doc_id: String,
}

/// 更新单行实盘数量输入
pub struct UpdateInventoryCountLineInput {
    pub count_doc_id: String,
    pub line_no: i32,

    pub counted_qty: Decimal,
    pub difference_reason: Option<String>,
    pub remark: Option<String>,

    pub operator: String,
}

/// 批量更新实盘数量输入
pub struct BatchUpdateInventoryCountLinesInput {
    pub count_doc_id: String,
    pub lines: Vec<BatchUpdateInventoryCountLineItem>,
    pub operator: String,
}

pub struct BatchUpdateInventoryCountLineItem {
    pub line_no: i32,
    pub counted_qty: Decimal,
    pub difference_reason: Option<String>,
    pub remark: Option<String>,
}

/// 提交盘点单输入
pub struct SubmitInventoryCountInput {
    pub count_doc_id: String,
    pub operator: String,
    pub remark: Option<String>,
}

/// 审核盘点单输入
pub struct ApproveInventoryCountInput {
    pub count_doc_id: String,

    /// true = 审核通过
    /// false = 退回重盘
    pub approved: bool,

    pub operator: String,
    pub remark: Option<String>,
}

/// 盘点过账输入
pub struct PostInventoryCountInput {
    pub count_doc_id: String,
    pub posting_date: OffsetDateTime,
    pub operator: String,
    pub remark: Option<String>,
}

/// 关闭盘点单输入
pub struct CloseInventoryCountInput {
    pub count_doc_id: String,
    pub operator: String,
    pub remark: Option<String>,
}

/// 取消盘点单输入
pub struct CancelInventoryCountInput {
    pub count_doc_id: String,
    pub operator: String,
    pub remark: Option<String>,
}

/// 从盘点范围生成明细时使用
#[derive(Debug, Clone)]
pub struct InventoryCountScopeFilter {
    pub count_scope: InventoryCountScope,
    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
}
