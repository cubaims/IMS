use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl PageQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        (page as i64 - 1) * self.limit()
    }
}

impl Default for PageQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrentStockQuery {
    pub material_id: Option<String>,
    pub bin_code: Option<String>,
    pub batch_number: Option<String>,
    pub zone: Option<String>,
    pub quality_status: Option<String>,
    pub only_available: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl CurrentStockQuery {
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page: self.page,
            page_size: self.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryTransactionQuery {
    pub transaction_id: Option<String>,
    pub material_id: Option<String>,
    pub movement_type: Option<String>,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub reference_doc: Option<String>,
    pub operator: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl InventoryTransactionQuery {
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page: self.page,
            page_size: self.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchQuery {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub only_available: Option<bool>,
    pub only_expiring: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl BatchQuery {
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page: self.page,
            page_size: self.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchHistoryQuery {
    pub event_type: Option<String>,
    pub operator: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl BatchHistoryQuery {
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page: self.page,
            page_size: self.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapHistoryQuery {
    pub material_id: Option<String>,
    pub transaction_id: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl MapHistoryQuery {
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page: self.page,
            page_size: self.page_size,
        }
    }
}