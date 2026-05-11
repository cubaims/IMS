use serde::{Deserialize, Serialize};

use crate::domain::TraceQueryOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilityResponse {
    pub module: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceQueryParams {
    pub max_depth: Option<u32>,
    pub movement_limit: Option<u32>,
    pub event_limit: Option<u32>,
    pub quality_limit: Option<u32>,
    pub include_genealogy: Option<bool>,
    pub include_inventory: Option<bool>,
    pub include_history: Option<bool>,
    pub include_quality: Option<bool>,
}

impl TraceQueryParams {
    pub fn options(self) -> TraceQueryOptions {
        let defaults = TraceQueryOptions::default();

        TraceQueryOptions {
            max_depth: self.max_depth.unwrap_or(defaults.max_depth),
            movement_limit: self.movement_limit.unwrap_or(defaults.movement_limit),
            event_limit: self.event_limit.unwrap_or(defaults.event_limit),
            quality_limit: self.quality_limit.unwrap_or(defaults.quality_limit),
            include_genealogy: self.include_genealogy.unwrap_or(defaults.include_genealogy),
            include_inventory: self.include_inventory.unwrap_or(defaults.include_inventory),
            include_history: self.include_history.unwrap_or(defaults.include_history),
            include_quality: self.include_quality.unwrap_or(defaults.include_quality),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SerialTraceQueryParams {
    pub max_depth: Option<u32>,
    pub movement_limit: Option<u32>,
    pub event_limit: Option<u32>,
    pub quality_limit: Option<u32>,
    pub include_genealogy: Option<bool>,
    pub include_inventory: Option<bool>,
    pub include_history: Option<bool>,
    pub include_quality: Option<bool>,
    pub include_batch_context: Option<bool>,
}

impl SerialTraceQueryParams {
    pub fn include_batch_context(&self) -> bool {
        self.include_batch_context.unwrap_or(true)
    }

    pub fn options(self) -> TraceQueryOptions {
        TraceQueryParams {
            max_depth: self.max_depth,
            movement_limit: self.movement_limit,
            event_limit: self.event_limit,
            quality_limit: self.quality_limit,
            include_genealogy: self.include_genealogy,
            include_inventory: self.include_inventory,
            include_history: self.include_history,
            include_quality: self.include_quality,
        }
        .options()
    }
}
