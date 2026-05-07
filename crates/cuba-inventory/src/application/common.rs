use serde::{Deserialize, Serialize};

/// 通用分页查询参数
///
/// 如果项目里 cuba-shared 已经有 PageQuery，可以后续替换成共享结构。
#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    /// 当前页，从 1 开始
    pub page: Option<u64>,

    /// 每页数量
    pub page_size: Option<u64>,

    /// 排序字段
    pub sort_by: Option<String>,

    /// asc / desc
    pub sort_order: Option<String>,
}

impl PageQuery {
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u64 {
        self.page_size.unwrap_or(20).clamp(1, 200)
    }

    pub fn offset(&self) -> u64 {
        (self.page() - 1) * self.page_size()
    }
}

/// 通用分页返回结构
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: u64, page_size: u64, total: u64) -> Self {
        Self {
            items,
            page,
            page_size,
            total,
        }
    }
}
