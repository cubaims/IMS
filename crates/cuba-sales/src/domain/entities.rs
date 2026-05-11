use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::domain::SalesDomainError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub so_id: String,
    pub customer_id: String,
    pub so_date: Date,
    pub required_date: Option<Date>,
    pub status: SalesOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub lines: Vec<SalesOrderLine>,
}

impl SalesOrder {
    pub fn ship_line(&mut self, line_no: i32, shipment_qty: i32) -> Result<(), SalesDomainError> {
        self.status.ensure_can_ship()?;

        let line = self
            .lines
            .iter_mut()
            .find(|line| line.line_no == line_no)
            .ok_or(SalesDomainError::SalesOrderLineNotFound)?;

        line.ship(shipment_qty)?;
        self.refresh_status_from_lines()?;

        Ok(())
    }

    pub fn refresh_status_from_lines(&mut self) -> Result<(), SalesDomainError> {
        self.status = Self::status_from_lines(&self.lines)?;
        Ok(())
    }

    pub fn status_from_lines(
        lines: &[SalesOrderLine],
    ) -> Result<SalesOrderStatus, SalesDomainError> {
        let active_lines = lines
            .iter()
            .filter(|line| line.line_status != SalesLineStatus::Cancelled)
            .collect::<Vec<_>>();

        if active_lines.is_empty() {
            return Err(SalesDomainError::EmptySalesOrder);
        }

        let ordered_qty: i32 = active_lines.iter().map(|line| line.ordered_qty).sum();
        let shipped_qty: i32 = active_lines.iter().map(|line| line.shipped_qty).sum();

        if ordered_qty <= 0 {
            return Err(SalesDomainError::InvalidQuantity);
        }

        if shipped_qty >= ordered_qty {
            Ok(SalesOrderStatus::Shipped)
        } else if shipped_qty > 0 {
            Ok(SalesOrderStatus::PartiallyShipped)
        } else {
            Ok(SalesOrderStatus::Open)
        }
    }

    pub fn close(&mut self) -> Result<(), SalesDomainError> {
        self.status.ensure_can_close()?;
        self.status = SalesOrderStatus::Closed;

        for line in &mut self.lines {
            if line.line_status != SalesLineStatus::Completed {
                line.line_status = SalesLineStatus::Cancelled;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderLine {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub shipped_qty: i32,
    pub unit_price: Decimal,
    pub from_bin: Option<String>,
    pub line_status: SalesLineStatus,
}

impl SalesOrderLine {
    pub fn open_qty(&self) -> i32 {
        self.ordered_qty - self.shipped_qty
    }

    pub fn can_ship(&self, shipment_qty: i32) -> bool {
        self.line_status.can_ship() && shipment_qty > 0 && shipment_qty <= self.open_qty()
    }

    pub fn ship(&mut self, shipment_qty: i32) -> Result<(), SalesDomainError> {
        if !self.line_status.can_ship() {
            return Err(SalesDomainError::InvalidSalesOrderStatus);
        }

        let quantity = crate::domain::SalesQuantity::new(shipment_qty)?;

        if quantity.value() > self.open_qty() {
            return Err(SalesDomainError::ShipmentQuantityExceeded);
        }

        self.shipped_qty += quantity.value();
        self.line_status = SalesLineStatus::after_shipment(self.ordered_qty, self.shipped_qty)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesOrderStatus {
    Draft,
    Open,
    PartiallyShipped,
    Shipped,
    Closed,
    Cancelled,
}

impl SalesOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Open => "已审批",
            Self::PartiallyShipped => "部分发货",
            Self::Shipped => "完成",
            Self::Closed => "取消",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Result<Self, SalesDomainError> {
        match value {
            "草稿" => Ok(Self::Draft),
            "已审批" => Ok(Self::Open),
            "部分发货" => Ok(Self::PartiallyShipped),
            "完成" => Ok(Self::Shipped),
            "取消" => Ok(Self::Cancelled),
            _ => Err(SalesDomainError::InvalidSalesOrderStatus),
        }
    }

    pub fn can_ship(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyShipped)
    }

    pub fn ensure_can_ship(self) -> Result<(), SalesDomainError> {
        if self.can_ship() {
            Ok(())
        } else {
            Err(SalesDomainError::InvalidSalesOrderStatus)
        }
    }

    pub fn can_close(self) -> bool {
        !matches!(self, Self::Shipped | Self::Closed | Self::Cancelled)
    }

    pub fn ensure_can_close(self) -> Result<(), SalesDomainError> {
        if self.can_close() {
            Ok(())
        } else {
            Err(SalesDomainError::InvalidSalesOrderStatus)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesLineStatus {
    Open,
    PartiallyShipped,
    Completed,
    Cancelled,
}

impl SalesLineStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Open => "待发货",
            Self::PartiallyShipped => "部分发货",
            Self::Completed => "完成",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Result<Self, SalesDomainError> {
        match value {
            "待发货" => Ok(Self::Open),
            "部分发货" => Ok(Self::PartiallyShipped),
            "完成" => Ok(Self::Completed),
            "取消" => Ok(Self::Cancelled),
            _ => Err(SalesDomainError::InvalidSalesOrderStatus),
        }
    }

    pub fn can_ship(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyShipped)
    }

    pub fn after_shipment(ordered_qty: i32, shipped_qty: i32) -> Result<Self, SalesDomainError> {
        if ordered_qty <= 0 || shipped_qty < 0 {
            return Err(SalesDomainError::InvalidQuantity);
        }

        if shipped_qty > ordered_qty {
            return Err(SalesDomainError::ShipmentQuantityExceeded);
        }

        if shipped_qty == ordered_qty {
            Ok(Self::Completed)
        } else if shipped_qty > 0 {
            Ok(Self::PartiallyShipped)
        } else {
            Ok(Self::Open)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PickStrategy {
    Fefo,
    Manual,
}

impl PickStrategy {
    pub fn is_fefo(&self) -> bool {
        matches!(self, Self::Fefo)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesShipmentResult {
    pub so_id: String,
    pub status: String,
    pub transactions: Vec<SalesShipmentTransaction>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesShipmentTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FefoPickLine {
    pub material_id: String,
    pub requested_qty: i32,
    pub picks: Vec<FefoPickBatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FefoPickBatch {
    pub batch_number: String,
    pub bin_code: String,
    pub pick_qty: i32,
    pub expiry_date: Option<Date>,
    pub available_qty: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(line_no: i32, ordered_qty: i32, shipped_qty: i32) -> SalesOrderLine {
        SalesOrderLine {
            line_no,
            material_id: "MAT-001".to_string(),
            ordered_qty,
            shipped_qty,
            unit_price: Decimal::ONE,
            from_bin: Some("BIN-A".to_string()),
            line_status: SalesLineStatus::after_shipment(ordered_qty, shipped_qty)
                .expect("valid line status"),
        }
    }

    #[test]
    fn shipping_partial_quantity_updates_open_line_status() {
        let mut line = line(10, 100, 0);

        line.ship(25).expect("line can ship");

        assert_eq!(line.shipped_qty, 25);
        assert_eq!(line.line_status, SalesLineStatus::PartiallyShipped);
    }

    #[test]
    fn shipping_remaining_quantity_completes_line() {
        let mut line = line(10, 100, 25);

        line.ship(75).expect("line can ship remaining quantity");

        assert_eq!(line.shipped_qty, 100);
        assert_eq!(line.line_status, SalesLineStatus::Completed);
    }

    #[test]
    fn shipment_quantity_cannot_exceed_open_quantity() {
        let mut line = line(10, 100, 25);

        let err = line
            .ship(76)
            .expect_err("shipment cannot exceed open quantity");

        assert!(matches!(err, SalesDomainError::ShipmentQuantityExceeded));
    }

    #[test]
    fn order_status_is_derived_from_active_lines() {
        let status = SalesOrder::status_from_lines(&[line(10, 100, 100), line(20, 50, 20)])
            .expect("active lines derive status");

        assert_eq!(status, SalesOrderStatus::PartiallyShipped);

        let status = SalesOrder::status_from_lines(&[line(10, 100, 100), line(20, 50, 50)])
            .expect("completed lines derive status");

        assert_eq!(status, SalesOrderStatus::Shipped);
    }
}
