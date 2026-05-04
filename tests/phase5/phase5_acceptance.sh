#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

require_jq() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required"
    exit 1
  fi
}

assert_success() {
  local response="$1"
  local success
  success=$(echo "$response" | jq -r '.success')
  
  if [ "$success" != "true" ]; then
    echo "Expected success=true, got:"
    echo "$response" | jq
    exit 1
  fi
}

require_jq

echo "========== Phase 5 Acceptance =========="

echo "1. Health check"
curl -s "$BASE_URL/health" | jq

echo "2. Create PO"
PO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/purchase-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "supplier_id": "SUP-001",
    "expected_date": "2026-05-15",
    "remark": "phase 5 po acceptance",
    "lines": [
      {
        "line_no": 10,
        "material_id": "CG001",
        "ordered_qty": 100,
        "unit_price": 12.5,
        "expected_bin": "RM-A01"
      }
    ]
  }')

echo "$PO_RESPONSE" | jq
assert_success "$PO_RESPONSE"

PO_ID=$(echo "$PO_RESPONSE" | jq -r '.data.po_id')

if [ -z "$PO_ID" ] || [ "$PO_ID" = "null" ]; then
  echo "PO_ID missing"
  exit 1
fi

echo "3. Post PO receipt"
RECEIPT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/purchase-orders/$PO_ID/receipt" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "posting_date": "2026-05-04T10:00:00Z",
    "remark": "phase 5 receipt acceptance",
    "lines": [
      {
        "line_no": 10,
        "receipt_qty": 100,
        "batch_number": "BATCH-CG001-P5-ACCEPT",
        "to_bin": "RM-A01"
      }
    ]
  }')

echo "$RECEIPT_RESPONSE" | jq
assert_success "$RECEIPT_RESPONSE"

RECEIPT_MOVEMENT=$(echo "$RECEIPT_RESPONSE" | jq -r '.data.transactions[0].movement_type')

if [ "$RECEIPT_MOVEMENT" != "101" ]; then
  echo "Expected movement_type=101, got $RECEIPT_MOVEMENT"
  exit 1
fi

echo "4. Create SO"
SO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "CUST-001",
    "required_date": "2026-05-20",
    "remark": "phase 5 so acceptance",
    "lines": [
      {
        "line_no": 10,
        "material_id": "CG001",
        "ordered_qty": 20,
        "unit_price": 20.0,
        "from_bin": "RM-A01"
      }
    ]
  }')

echo "$SO_RESPONSE" | jq
assert_success "$SO_RESPONSE"

SO_ID=$(echo "$SO_RESPONSE" | jq -r '.data.so_id')

if [ -z "$SO_ID" ] || [ "$SO_ID" = "null" ]; then
  echo "SO_ID missing"
  exit 1
fi

echo "5. Preview FEFO"
FEFO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/pick-preview" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 20
      }
    ]
  }')

echo "$FEFO_RESPONSE" | jq
assert_success "$FEFO_RESPONSE"

FEFO_PICK_QTY=$(echo "$FEFO_RESPONSE" | jq -r '.data.lines[0].picks[0].pick_qty')

if [ "$FEFO_PICK_QTY" = "null" ] || [ "$FEFO_PICK_QTY" -le 0 ]; then
  echo "FEFO pick failed"
  exit 1
fi

echo "6. Post SO shipment"
SHIPMENT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/shipment" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "posting_date": "2026-05-04T11:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "phase 5 shipment acceptance",
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 20
      }
    ]
  }')

echo "$SHIPMENT_RESPONSE" | jq
assert_success "$SHIPMENT_RESPONSE"

SHIPMENT_MOVEMENT=$(echo "$SHIPMENT_RESPONSE" | jq -r '.data.transactions[0].movement_type')

if [ "$SHIPMENT_MOVEMENT" != "261" ]; then
  echo "Expected movement_type=261, got $SHIPMENT_MOVEMENT"
  exit 1
fi

echo "7. Refresh reports"
REPORT_REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/reports/refresh" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$REPORT_REFRESH_RESPONSE" | jq
assert_success "$REPORT_REFRESH_RESPONSE"

echo "8. Query current stock"
CURRENT_STOCK_RESPONSE=$(curl -s "$BASE_URL/api/reports/current-stock" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$CURRENT_STOCK_RESPONSE" | jq

echo "========== Phase 5 Acceptance Passed =========="
