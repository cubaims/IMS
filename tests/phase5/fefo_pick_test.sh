#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "Create SO for FEFO preview"

SO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "CUST-001",
    "required_date": "2026-05-20",
    "remark": "fefo pick test",
    "lines": [
      {
        "line_no": 10,
        "material_id": "CG001",
        "ordered_qty": 5,
        "unit_price": 20.0,
        "from_bin": "RM-A01"
      }
    ]
  }')

echo "$SO_RESPONSE" | jq

SO_ID=$(echo "$SO_RESPONSE" | jq -r '.data.so_id')

echo "Preview FEFO"

RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/pick-preview" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 5
      }
    ]
  }')

echo "$RESPONSE" | jq

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
PICK_QTY=$(echo "$RESPONSE" | jq -r '.data.lines[0].picks[0].pick_qty')

if [ "$SUCCESS" != "true" ]; then
  echo "FEFO preview failed"
  exit 1
fi

if [ "$PICK_QTY" = "null" ] || [ "$PICK_QTY" -le 0 ]; then
  echo "FEFO pick quantity invalid"
  exit 1
fi

echo "FEFO pick test passed"
