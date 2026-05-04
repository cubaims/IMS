#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "Create PO"

PO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/purchase-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "supplier_id": "SUP-001",
    "expected_date": "2026-05-15",
    "remark": "po receipt test",
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

PO_ID=$(echo "$PO_RESPONSE" | jq -r '.data.po_id')

echo "Post PO receipt"

RESPONSE=$(curl -s -X POST "$BASE_URL/api/purchase-orders/$PO_ID/receipt" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "posting_date": "2026-05-04T10:00:00Z",
    "remark": "po receipt test",
    "lines": [
      {
        "line_no": 10,
        "receipt_qty": 100,
        "batch_number": "BATCH-CG001-PO-TEST",
        "to_bin": "RM-A01"
      }
    ]
  }')

echo "$RESPONSE" | jq

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
MOVEMENT=$(echo "$RESPONSE" | jq -r '.data.transactions[0].movement_type')
STALE=$(echo "$RESPONSE" | jq -r '.data.reports_stale')

if [ "$SUCCESS" != "true" ]; then
  echo "PO receipt failed"
  exit 1
fi

if [ "$MOVEMENT" != "101" ]; then
  echo "Expected 101 movement, got $MOVEMENT"
  exit 1
fi

if [ "$STALE" != "true" ]; then
  echo "Expected reports_stale=true"
  exit 1
fi

echo "PO receipt test passed"
