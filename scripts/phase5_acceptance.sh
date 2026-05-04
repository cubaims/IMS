#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "== Phase 5 Acceptance Test =="
echo ""

echo "1. Create PO"
PO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/purchase-orders" \
  "${AUTH_HEADER[@]}" \
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

echo "$PO_RESPONSE"
echo ""

PO_ID=$(echo "$PO_RESPONSE" | jq -r '.data.po_id')

echo "2. Post PO receipt"
curl -s -X POST "$BASE_URL/api/purchase-orders/$PO_ID/receipt" \
  "${AUTH_HEADER[@]}" \
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
  }' | jq
echo ""

echo "3. Create SO"
SO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders" \
  "${AUTH_HEADER[@]}" \
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

echo "$SO_RESPONSE"
echo ""

SO_ID=$(echo "$SO_RESPONSE" | jq -r '.data.so_id')

echo "4. Preview FEFO"
curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/pick-preview" \
  "${AUTH_HEADER[@]}" \
  -H "Content-Type: application/json" \
  -d '{
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 20
      }
    ]
  }' | jq
echo ""

echo "5. Post SO shipment"
curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/shipment" \
  "${AUTH_HEADER[@]}" \
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
  }' | jq
echo ""

echo "6. Refresh reports"
curl -s -X POST "$BASE_URL/api/reports/refresh" \
  "${AUTH_HEADER[@]}" | jq
echo ""

echo "7. Query current stock"
curl -s "$BASE_URL/api/reports/current-stock" \
  "${AUTH_HEADER[@]}" | jq
echo ""

echo "== Phase 5 acceptance finished =="
