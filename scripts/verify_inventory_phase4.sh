#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${IMS_BASE_URL:-http://localhost:8080}"
TOKEN="${IMS_AUTH_TOKEN:-}"
SUFFIX="$(date +%Y%m%d%H%M%S)"
MATERIAL_ID="${IMS_TEST_MATERIAL_ID:-CG001}"
BATCH_NUMBER="${IMS_TEST_BATCH_NUMBER:-B-CG-20260401}"

auth_header=()
if [[ -n "$TOKEN" ]]; then
  auth_header=(-H "Authorization: Bearer ${TOKEN}")
fi

post_json() {
  local path="$1"
  local body="$2"

  curl -sS -X POST "${BASE_URL}${path}" \
    "${auth_header[@]}" \
    -H "Content-Type: application/json" \
    -H "x-user-name: phase4-script" \
    -d "$body"
}

get_json() {
  local path="$1"

  curl -sS -X GET "${BASE_URL}${path}" \
    "${auth_header[@]}" \
    -H "x-user-name: phase4-script"
}

echo "== Phase 4 Inventory Verification =="
echo "BASE_URL=${BASE_URL}"
echo "MATERIAL_ID=${MATERIAL_ID}"
echo "BATCH_NUMBER=${BATCH_NUMBER}"

echo
echo "1) 101 Receipt"
post_json "/api/inventory/post" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"movement_type\":\"101\",
  \"quantity\":100,
  \"to_bin\":\"RM-A01\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-101\",
  \"quality_status\":\"合格\",
  \"unit_price\":18.50,
  \"remark\":\"phase4 101 receipt script\"
}" | jq .

echo
echo "2) 311 Transfer"
post_json "/api/inventory/transfer" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"quantity\":20,
  \"from_bin\":\"RM-A01\",
  \"to_bin\":\"RM-A02\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-311\",
  \"quality_status\":\"合格\",
  \"remark\":\"phase4 311 transfer script\"
}" | jq .

echo
echo "3) 261 Issue"
post_json "/api/inventory/post" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"movement_type\":\"261\",
  \"quantity\":10,
  \"from_bin\":\"RM-A02\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-261\",
  \"quality_status\":\"合格\",
  \"remark\":\"phase4 261 issue script\"
}" | jq .

echo
echo "4) 701 Count Gain"
post_json "/api/inventory/post" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"movement_type\":\"701\",
  \"quantity\":5,
  \"to_bin\":\"RM-A02\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-701\",
  \"quality_status\":\"合格\",
  \"remark\":\"phase4 701 gain script\"
}" | jq .

echo
echo "5) 702 Count Loss"
post_json "/api/inventory/post" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"movement_type\":\"702\",
  \"quantity\":5,
  \"from_bin\":\"RM-A02\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-702\",
  \"quality_status\":\"合格\",
  \"remark\":\"phase4 702 loss script\"
}" | jq .

echo
echo "6) 999 Scrap"
post_json "/api/inventory/post" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"movement_type\":\"999\",
  \"quantity\":2,
  \"from_bin\":\"RM-A02\",
  \"batch_number\":\"${BATCH_NUMBER}\",
  \"reference_doc\":\"PH4-${SUFFIX}-999\",
  \"quality_status\":\"合格\",
  \"remark\":\"phase4 999 scrap script\"
}" | jq .

echo
echo "7) Current Stock"
get_json "/api/inventory/current?material_id=${MATERIAL_ID}&batch_number=${BATCH_NUMBER}" | jq .

echo
echo "8) Bin Stock"
get_json "/api/inventory/bin-stock?material_id=${MATERIAL_ID}&batch_number=${BATCH_NUMBER}" | jq .

echo
echo "9) Transactions"
get_json "/api/inventory/transactions?material_id=${MATERIAL_ID}&batch_number=${BATCH_NUMBER}" | jq .

echo
echo "10) Batch"
get_json "/api/inventory/batches/${BATCH_NUMBER}" | jq .

echo
echo "11) Batch History"
get_json "/api/inventory/batches/${BATCH_NUMBER}/history" | jq .

echo
echo "12) MAP History"
get_json "/api/inventory/materials/${MATERIAL_ID}/map-history" | jq .

echo
echo "13) FEFO"
post_json "/api/inventory/pick-batch-fefo" "{
  \"material_id\":\"${MATERIAL_ID}\",
  \"quantity\":1,
  \"quality_status\":\"合格\"
}" | jq .

echo
echo "Phase 4 verification finished."
