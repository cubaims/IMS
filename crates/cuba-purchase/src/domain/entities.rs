use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::domain::PurchaseDomainError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub po_id: String,
    pub supplier_id: String,
    pub po_date: Date,
    pub expected_date: Option<Date>,
    pub status: PurchaseOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub lines: Vec<PurchaseOrderLine>,
}

impl PurchaseOrder {
    pub fn receive_line(
        &mut self,
        line_no: i32,
        receipt_qty: i32,
    ) -> Result<(), PurchaseDomainError> {
        self.status.ensure_can_receive()?;

        let line = self
            .lines
            .iter_mut()
            .find(|line| line.line_no == line_no)
            .ok_or(PurchaseDomainError::PurchaseOrderLineNotFound)?;

        line.receive(receipt_qty)?;
        self.refresh_status_from_lines()?;

        Ok(())
    }

    pub fn refresh_status_from_lines(&mut self) -> Result<(), PurchaseDomainError> {
        self.status = Self::status_from_lines(&self.lines)?;
        Ok(())
    }

    pub fn status_from_lines(
        lines: &[PurchaseOrderLine],
    ) -> Result<PurchaseOrderStatus, PurchaseDomainError> {
        let active_lines = lines
            .iter()
            .filter(|line| line.line_status != PurchaseLineStatus::Cancelled)
            .collect::<Vec<_>>();

        if active_lines.is_empty() {
            return Err(PurchaseDomainError::EmptyPurchaseOrder);
        }

        let ordered_qty: i32 = active_lines.iter().map(|line| line.ordered_qty).sum();
        let received_qty: i32 = active_lines.iter().map(|line| line.received_qty).sum();

        if ordered_qty <= 0 {
            return Err(PurchaseDomainError::InvalidQuantity);
        }

        if received_qty >= ordered_qty {
            Ok(PurchaseOrderStatus::Received)
        } else if received_qty > 0 {
            Ok(PurchaseOrderStatus::PartiallyReceived)
        } else {
            Ok(PurchaseOrderStatus::Open)
        }
    }

    pub fn close(&mut self) -> Result<(), PurchaseDomainError> {
        self.status.ensure_can_close()?;
        self.status = PurchaseOrderStatus::Closed;

        for line in &mut self.lines {
            if line.line_status != PurchaseLineStatus::Completed {
                line.line_status = PurchaseLineStatus::Cancelled;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub received_qty: i32,
    pub unit_price: Decimal,
    pub expected_bin: Option<String>,
    pub line_status: PurchaseLineStatus,
}

impl PurchaseOrderLine {
    pub fn open_qty(&self) -> i32 {
        self.ordered_qty - self.received_qty
    }

    pub fn can_receive(&self, receipt_qty: i32) -> bool {
        self.line_status.can_receive() && receipt_qty > 0 && receipt_qty <= self.open_qty()
    }

    pub fn receive(&mut self, receipt_qty: i32) -> Result<(), PurchaseDomainError> {
        if !self.line_status.can_receive() {
            return Err(PurchaseDomainError::InvalidPurchaseOrderStatus);
        }

        let quantity = crate::domain::PurchaseQuantity::new(receipt_qty)?;

        if quantity.value() > self.open_qty() {
            return Err(PurchaseDomainError::ReceiptQuantityExceeded);
        }

        self.received_qty += quantity.value();
        self.line_status = PurchaseLineStatus::after_receipt(self.ordered_qty, self.received_qty)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseOrderStatus {
    Draft,
    Open,
    PartiallyReceived,
    Received,
    Closed,
    Cancelled,
}

impl PurchaseOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Open => "已审批",
            Self::PartiallyReceived => "部分到货",
            Self::Received => "完成",
            Self::Closed => "取消",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Result<Self, PurchaseDomainError> {
        match value {
            "草稿" => Ok(Self::Draft),
            "已审批" => Ok(Self::Open),
            "部分到货" => Ok(Self::PartiallyReceived),
            "完成" => Ok(Self::Received),
            "取消" => Ok(Self::Cancelled),
            _ => Err(PurchaseDomainError::InvalidPurchaseOrderStatus),
        }
    }

    pub fn can_receive(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyReceived)
    }

    pub fn ensure_can_receive(self) -> Result<(), PurchaseDomainError> {
        if self.can_receive() {
            Ok(())
        } else {
            Err(PurchaseDomainError::InvalidPurchaseOrderStatus)
        }
    }

    pub fn can_close(self) -> bool {
        !matches!(self, Self::Received | Self::Closed | Self::Cancelled)
    }

    pub fn ensure_can_close(self) -> Result<(), PurchaseDomainError> {
        if self.can_close() {
            Ok(())
        } else {
            Err(PurchaseDomainError::InvalidPurchaseOrderStatus)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseLineStatus {
    Open,
    PartiallyReceived,
    Completed,
    Cancelled,
}

impl PurchaseLineStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Open => "待到货",
            Self::PartiallyReceived => "部分到货",
            Self::Completed => "完成",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Result<Self, PurchaseDomainError> {
        match value {
            "待到货" => Ok(Self::Open),
            "部分到货" => Ok(Self::PartiallyReceived),
            "完成" => Ok(Self::Completed),
            "取消" => Ok(Self::Cancelled),
            _ => Err(PurchaseDomainError::InvalidPurchaseOrderStatus),
        }
    }

    pub fn can_receive(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyReceived)
    }

    pub fn after_receipt(ordered_qty: i32, received_qty: i32) -> Result<Self, PurchaseDomainError> {
        if ordered_qty <= 0 || received_qty < 0 {
            return Err(PurchaseDomainError::InvalidQuantity);
        }

        if received_qty > ordered_qty {
            return Err(PurchaseDomainError::ReceiptQuantityExceeded);
        }

        if received_qty == ordered_qty {
            Ok(Self::Completed)
        } else if received_qty > 0 {
            Ok(Self::PartiallyReceived)
        } else {
            Ok(Self::Open)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptResult {
    pub po_id: String,
    pub status: String,
    pub transactions: Vec<PurchaseReceiptTransaction>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub to_bin: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(line_no: i32, ordered_qty: i32, received_qty: i32) -> PurchaseOrderLine {
        PurchaseOrderLine {
            line_no,
            material_id: "MAT-001".to_string(),
            ordered_qty,
            received_qty,
            unit_price: Decimal::ONE,
            expected_bin: Some("BIN-A".to_string()),
            line_status: PurchaseLineStatus::after_receipt(ordered_qty, received_qty)
                .expect("valid line status"),
        }
    }

    #[test]
    fn receiving_partial_quantity_updates_open_line_status() {
        let mut line = line(10, 100, 0);

        line.receive(30).expect("line can receive");

        assert_eq!(line.received_qty, 30);
        assert_eq!(line.line_status, PurchaseLineStatus::PartiallyReceived);
    }

    #[test]
    fn receiving_remaining_quantity_completes_line() {
        let mut line = line(10, 100, 40);

        line.receive(60)
            .expect("line can receive remaining quantity");

        assert_eq!(line.received_qty, 100);
        assert_eq!(line.line_status, PurchaseLineStatus::Completed);
    }

    #[test]
    fn receipt_quantity_cannot_exceed_open_quantity() {
        let mut line = line(10, 100, 40);

        let err = line
            .receive(61)
            .expect_err("receipt cannot exceed open quantity");

        assert!(matches!(err, PurchaseDomainError::ReceiptQuantityExceeded));
    }

    #[test]
    fn order_status_is_derived_from_active_lines() {
        let status = PurchaseOrder::status_from_lines(&[line(10, 100, 100), line(20, 50, 20)])
            .expect("active lines derive status");

        assert_eq!(status, PurchaseOrderStatus::PartiallyReceived);

        let status = PurchaseOrder::status_from_lines(&[line(10, 100, 100), line(20, 50, 50)])
            .expect("completed lines derive status");

        assert_eq!(status, PurchaseOrderStatus::Received);
    }
}
