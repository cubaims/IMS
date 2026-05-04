#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "Create SO"

SO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "CUST-001",
    "required_date": "2026-05-20",
    "remark": "so shipment test",
    "lines": [
      {
        "line_no": 10,
        "material_id": "CG001",
        "ordered_qty": 10,
        "unit_price": 20.0,
        "from_bin": "RM-A01"
      }
    ]
  }')

echo "$SO_RESPONSE" | jq

SO_ID=$(echo "$SO_RESPONSE" | jq -r '.data.so_id')

echo "Post SO shipment"

RESPONSE=$(curl -s -X POST "$BASE_URL/api/sales-orders/$SO_ID/shipment" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "posting_date": "2026-05-04T11:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "so shipment test",
    "lines": [
      {
        "line_no": 10,
        "shipment_qty": 10
      }
    ]
  }')

echo "$RESPONSE" | jq

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
MOVEMENT=$(echo "$RESPONSE" | jq -r '.data.transactions[0].movement_type')
STALE=$(echo "$RESPONSE" | jq -r '.data.reports_stale')

if [ "$SUCCESS" != "true" ]; then
  echo "SO shipment failed"
  exit 1
fi

if [ "$MOVEMENT" != "261" ]; then
  echo "Expected 261 movement, got $MOVEMENT"
  exit 1
fi

if [ "$STALE" != "true" ]; then
  echo "Expected reports_stale=true"
  exit 1
fi

echo "SO shipment test passed"
