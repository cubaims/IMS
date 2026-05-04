#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "Create SO with excessive quantity"

SO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "CUST-001",
    "required_date": "2026-05-20",
    "remark": "insufficient stock test",
    "lines": [
      {
        "line_no": 10,
        "material_id": "CG001",
        "ordered_qty": 999999,
        "unit_price": 20.0,
        "from_bin": "RM-A01"
      }
    ]
  }')

echo "$SO_RESPONSE" | jq

SO_ID=$(echo "$SO_RESPONSE" | jq -r '.data.so_id')

echo "Try shipment with excessive quantity"

RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/shipment" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "posting_date": "2026-05-04T11:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "insufficient stock test",
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 999999
      }
    ]
  }')

echo "$RESPONSE" | jq

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
ERROR_CODE=$(echo "$RESPONSE" | jq -r '.error_code')

if [ "$SUCCESS" != "false" ]; then
  echo "Expected failure but got success"
  exit 1
fi

case "$ERROR_CODE" in
  "INSUFFICIENT_STOCK"|"NO_AVAILABLE_BATCH")
    echo "Insufficient stock blocked correctly: $ERROR_CODE"
    ;;
  *)
    echo "Unexpected error_code: $ERROR_CODE"
    exit 1
    ;;
esac

echo "Insufficient stock test passed"
