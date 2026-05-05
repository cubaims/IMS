#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

if [ -z "$TOKEN" ]; then
  echo "TOKEN is required. Example:"
  echo "TOKEN=xxx ./scripts/verify_phase6_production.sh"
  exit 1
fi

AUTH_HEADER="Authorization: Bearer ${TOKEN}"

echo "== Phase 6: BOM explosion =="

curl -sS -X POST "${BASE_URL}/api/production/bom-explosion" \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "quantity": 10,
    "merge_components": true
  }' | jq .

echo "== Phase 6: create production order =="

CREATE_RESPONSE=$(curl -sS -X POST "${BASE_URL}/api/production-orders" \
  -H "${AUTH_HEADER}" \
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

echo "$CREATE_RESPONSE" | jq .

ORDER_ID=$(echo "$CREATE_RESPONSE" | jq -r '.data.order_id')

if [ -z "$ORDER_ID" ] || [ "$ORDER_ID" = "null" ]; then
  echo "Failed to create production order"
  exit 1
fi

echo "ORDER_ID=${ORDER_ID}"

echo "== Phase 6: get production order =="

curl -sS "${BASE_URL}/api/production-orders/${ORDER_ID}" \
  -H "${AUTH_HEADER}" | jq .

echo "== Phase 6: get production order components =="

curl -sS "${BASE_URL}/api/production-orders/${ORDER_ID}/components" \
  -H "${AUTH_HEADER}" | jq .

echo "== Phase 6: release production order =="

curl -sS -X POST "${BASE_URL}/api/production-orders/${ORDER_ID}/release" \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  -d '{"remark":"release order"}' | jq .

FINISHED_BATCH="BATCH-FIN001-P6-$(date +%Y%m%d%H%M%S)"

echo "== Phase 6: complete production order =="

curl -sS -X POST "${BASE_URL}/api/production-orders/${ORDER_ID}/complete" \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  -d "{
    \"completed_qty\": 10,
    \"finished_batch_number\": \"${FINISHED_BATCH}\",
    \"finished_to_bin\": \"FG-A01\",
    \"posting_date\": \"2026-05-04T14:00:00Z\",
    \"pick_strategy\": \"FEFO\",
    \"remark\": \"phase 6 complete\"
  }" | jq .

echo "== Phase 6: genealogy =="

curl -sS "${BASE_URL}/api/production-orders/${ORDER_ID}/genealogy" \
  -H "${AUTH_HEADER}" | jq .

echo "== Phase 6: variance =="

curl -sS "${BASE_URL}/api/production-orders/${ORDER_ID}/variance" \
  -H "${AUTH_HEADER}" | jq .

echo "== Phase 6: refresh reports =="

curl -sS -X POST "${BASE_URL}/api/reports/refresh" \
  -H "${AUTH_HEADER}" | jq .

echo "== Phase 6 verification done =="