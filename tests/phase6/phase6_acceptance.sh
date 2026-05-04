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

echo "========== Phase 6 Production Order Acceptance =========="

echo "1. Health check"
curl -s "$BASE_URL/health" | jq

echo "2. Version check"
curl -s "$BASE_URL/api/version" | jq

echo "3. BOM explosion preview"
BOM_RESPONSE=$(curl -s -X POST "$BASE_URL/api/production/bom-explosion" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "quantity": 10,
    "merge_components": true
  }')

echo "$BOM_RESPONSE" | jq
assert_success "$BOM_RESPONSE"

COMPONENT_COUNT=$(echo "$BOM_RESPONSE" | jq -r '.data.components | length')
if [ "$COMPONENT_COUNT" -eq 0 ]; then
  echo "Expected components, got empty array"
  exit 1
fi

echo "4. Create production order"
PO_RESPONSE=$(curl -s -X POST "$BASE_URL/api/production-orders" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "bom_id": "BOM-FIN-A01",
    "planned_qty": 10,
    "work_center_id": "WC-ASSY-01",
    "planned_start_date": "2026-05-05",
    "planned_end_date": "2026-05-08",
    "remark": "phase 6 production order"
  }')

echo "$PO_RESPONSE" | jq
assert_success "$PO_RESPONSE"

ORDER_ID=$(echo "$PO_RESPONSE" | jq -r '.data.order_id')
if [ -z "$ORDER_ID" ] || [ "$ORDER_ID" = "null" ]; then
  echo "ORDER_ID missing"
  exit 1
fi

echo "ORDER_ID: $ORDER_ID"

echo "5. Query production order details"
ORDER_DETAIL=$(curl -s "$BASE_URL/api/production-orders/$ORDER_ID" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$ORDER_DETAIL" | jq
assert_success "$ORDER_DETAIL"

echo "6. Query production order components"
COMPONENTS=$(curl -s "$BASE_URL/api/production-orders/$ORDER_ID/components" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$COMPONENTS" | jq
assert_success "$COMPONENTS"

echo "7. Release production order"
RELEASE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/production-orders/$ORDER_ID/release" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "remark": "release phase 6 production order"
  }')

echo "$RELEASE_RESPONSE" | jq
assert_success "$RELEASE_RESPONSE"

RELEASE_STATUS=$(echo "$RELEASE_RESPONSE" | jq -r '.data.status')
if [ "$RELEASE_STATUS" != "RELEASED" ]; then
  echo "Expected status=RELEASED, got $RELEASE_STATUS"
  exit 1
fi

echo "8. Complete production order"
COMPLETE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/production-orders/$ORDER_ID/complete" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} \
  -H "Content-Type: application/json" \
  -d '{
    "completed_qty": 10,
    "finished_batch_number": "BATCH-FIN001-P6-001",
    "finished_to_bin": "FG-A01",
    "posting_date": "2026-05-04T14:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "phase 6 production complete"
  }')

echo "$COMPLETE_RESPONSE" | jq
assert_success "$COMPLETE_RESPONSE"

COMPLETE_STATUS=$(echo "$COMPLETE_RESPONSE" | jq -r '.data.status')
if [ "$COMPLETE_STATUS" != "COMPLETED" ]; then
  echo "Expected status=COMPLETED, got $COMPLETE_STATUS"
  exit 1
fi

FINISHED_MOVEMENT=$(echo "$COMPLETE_RESPONSE" | jq -r '.data.finished_transaction.movement_type')
if [ "$FINISHED_MOVEMENT" != "101" ]; then
  echo "Expected finished movement_type=101, got $FINISHED_MOVEMENT"
  exit 1
fi

COMPONENT_MOVEMENT=$(echo "$COMPLETE_RESPONSE" | jq -r '.data.component_transactions[0].movement_type')
if [ "$COMPONENT_MOVEMENT" != "261" ]; then
  echo "Expected component movement_type=261, got $COMPONENT_MOVEMENT"
  exit 1
fi

echo "9. Query batch genealogy"
GENEALOGY=$(curl -s "$BASE_URL/api/production-orders/$ORDER_ID/genealogy" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$GENEALOGY" | jq
assert_success "$GENEALOGY"

echo "10. Query finished batch components"
BATCH_COMPONENTS=$(curl -s "$BASE_URL/api/production/batches/BATCH-FIN001-P6-001/components" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$BATCH_COMPONENTS" | jq
assert_success "$BATCH_COMPONENTS"

echo "11. Query cost variance"
VARIANCE=$(curl -s "$BASE_URL/api/production-orders/$ORDER_ID/variance" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$VARIANCE" | jq
assert_success "$VARIANCE"

echo "12. Refresh reports"
REFRESH=$(curl -s -X POST "$BASE_URL/api/reports/refresh" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$REFRESH" | jq
assert_success "$REFRESH"

echo "13. Query current stock for FIN001"
STOCK=$(curl -s "$BASE_URL/api/reports/current-stock?material_id=FIN001" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$STOCK" | jq

echo "========== Phase 6 Acceptance Passed =========="
