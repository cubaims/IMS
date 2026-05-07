use serde::{Deserialize, Serialize};

/// 通用分页查询参数。
///
/// 放在 cuba-shared 中，供所有业务模块复用：
/// - cuba-quality
/// - cuba-inventory
/// - cuba-purchase
/// - cuba-sales
/// - cuba-reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageQuery {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

/// 通用分页结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 总记录数。
    pub total: u64,

    /// 当前页码。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        Self {
            items,
            total,
            page,
            page_size,
        }
    }
}

/// 排序方向。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SortOrder {
    Asc,
    Desc,
}
