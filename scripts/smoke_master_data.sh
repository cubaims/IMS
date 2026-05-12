#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"
READ_TOKEN="${READ_TOKEN:-}"

if [ -z "$TOKEN" ]; then
  echo "TOKEN is required. Example:"
  echo "TOKEN=xxx ./scripts/smoke_master_data.sh"
  exit 1
fi

if [ -z "$READ_TOKEN" ]; then
  echo "READ_TOKEN is required and must only grant master-data:read."
  echo "TOKEN=write_or_admin_token READ_TOKEN=read_only_token ./scripts/smoke_master_data.sh"
  exit 1
fi

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command not found: $1"
    exit 1
  fi
}

need curl
need jq

AUTH_HEADER="Authorization: Bearer ${TOKEN}"
READ_AUTH_HEADER="Authorization: Bearer ${READ_TOKEN}"
RUN_ID="$(date +%Y%m%d%H%M%S)"

MATERIAL_ID="RM${RUN_ID:4:10}"
FINISHED_ID="FN${RUN_ID:4:10}"
SUPPLIER_ID="SP${RUN_ID:4:10}"
CUSTOMER_ID="CU${RUN_ID:4:10}"
RAW_BIN="R${RUN_ID:6:8}"
FIN_BIN="F${RUN_ID:6:8}"
BOM_ID="BOM${RUN_ID:3:11}"
COPIED_BOM_ID="BMC${RUN_ID:3:10}"
VARIANT_CODE="V${RUN_ID:5:9}"
WORK_CENTER_ID="WC${RUN_ID:4:10}"
CHAR_ID="CH${RUN_ID:4:10}"
DEFECT_CODE="DF${RUN_ID:4:10}"

request() {
  local method="$1"
  local path="$2"
  local expected="$3"
  local body="${4:-}"
  local auth_header="${5:-$AUTH_HEADER}"
  local check_success="${6:-yes}"
  local response status payload

  if [ -n "$body" ]; then
    response="$(curl -sS -w '\n%{http_code}' -X "$method" "${BASE_URL}${path}" \
      -H "$auth_header" \
      -H "Content-Type: application/json" \
      -d "$body")"
  elif [ -n "$auth_header" ]; then
    response="$(curl -sS -w '\n%{http_code}' -X "$method" "${BASE_URL}${path}" \
      -H "$auth_header")"
  else
    response="$(curl -sS -w '\n%{http_code}' -X "$method" "${BASE_URL}${path}")"
  fi

  status="$(printf '%s' "$response" | tail -n 1)"
  payload="$(printf '%s' "$response" | sed '$d')"

  if [ "$status" != "$expected" ]; then
    echo "Unexpected status for $method $path: got $status expected $expected"
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  if [ "$expected" = "200" ] && [ "$check_success" = "yes" ]; then
    local success
    success="$(printf '%s\n' "$payload" | jq -r '.success // empty')"
    if [ "$success" != "true" ]; then
      echo "Expected success=true for $method $path"
      printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
      exit 1
    fi
  fi

  printf '%s\n' "$payload"
}

echo "== Phase 3 Master Data smoke test =="

echo "1. public OpenAPI document"
request GET "/api/openapi/master-data.json" 200 "" "" no >/tmp/ims-master-data-openapi.json
jq -e '.paths["/api/master-data/boms/{bom_id}/components/{component_id}"].patch' \
  /tmp/ims-master-data-openapi.json >/dev/null
jq -e '.paths["/api/master-data/boms/{bom_id}/copy"].post' \
  /tmp/ims-master-data-openapi.json >/dev/null

echo "2. permission guard rejects missing token"
request GET "/api/master-data/materials" 401 "" "" no >/dev/null

echo "3. permission guard rejects read-only token on write route"
request POST "/api/master-data/materials" 403 '{
  "material_id": "SHOULDNOTCREATE",
  "material_name": "Should Not Create",
  "material_type": "原材料",
  "base_unit": "PCS",
  "default_zone": "RM",
  "safety_stock": 0,
  "reorder_point": 0,
  "standard_price": "0.00",
  "map_price": "0.00"
}' "$READ_AUTH_HEADER" >/dev/null

echo "4. create raw and finished materials"
request POST "/api/master-data/materials" 200 "{
  \"material_id\": \"${MATERIAL_ID}\",
  \"material_name\": \"Smoke Raw Material\",
  \"material_type\": \"原材料\",
  \"base_unit\": \"PCS\",
  \"default_zone\": \"RM\",
  \"safety_stock\": 100,
  \"reorder_point\": 50,
  \"standard_price\": \"10.00\",
  \"map_price\": \"10.00\"
}" >/dev/null

request POST "/api/master-data/materials" 200 "{
  \"material_id\": \"${FINISHED_ID}\",
  \"material_name\": \"Smoke Finished Product\",
  \"material_type\": \"成品\",
  \"base_unit\": \"PCS\",
  \"default_zone\": \"FG\",
  \"safety_stock\": 20,
  \"reorder_point\": 10,
  \"standard_price\": \"100.00\",
  \"map_price\": \"100.00\"
}" >/dev/null

request GET "/api/master-data/materials/${MATERIAL_ID}" 200 >/dev/null

echo "5. create bins"
request POST "/api/master-data/bins" 200 "{
  \"bin_code\": \"${RAW_BIN}\",
  \"zone\": \"RM\",
  \"bin_type\": \"普通货位\",
  \"capacity\": 10000,
  \"notes\": \"Phase 3 smoke raw bin\"
}" >/dev/null

request POST "/api/master-data/bins" 200 "{
  \"bin_code\": \"${FIN_BIN}\",
  \"zone\": \"FG\",
  \"bin_type\": \"普通货位\",
  \"capacity\": 10000,
  \"notes\": \"Phase 3 smoke finished bin\"
}" >/dev/null

echo "6. create supplier and material-supplier relationship"
request POST "/api/master-data/suppliers" 200 "{
  \"supplier_id\": \"${SUPPLIER_ID}\",
  \"supplier_name\": \"Smoke Supplier\",
  \"contact_person\": \"Tester\",
  \"phone\": \"13800000000\",
  \"email\": \"supplier@example.com\",
  \"address\": \"Test Address\",
  \"quality_rating\": \"A\"
}" >/dev/null

request POST "/api/master-data/materials/${MATERIAL_ID}/suppliers" 200 "{
  \"material_id\": \"${MATERIAL_ID}\",
  \"supplier_id\": \"${SUPPLIER_ID}\",
  \"is_primary\": false,
  \"supplier_material_code\": \"SUP-${MATERIAL_ID}\",
  \"purchase_price\": \"9.50\",
  \"currency\": \"CNY\",
  \"lead_time_days\": 7,
  \"moq\": 100,
  \"quality_rating\": \"A\"
}" >/dev/null

request POST "/api/master-data/materials/${MATERIAL_ID}/suppliers/${SUPPLIER_ID}/primary" 200 >/dev/null
request DELETE "/api/master-data/materials/${MATERIAL_ID}/suppliers/${SUPPLIER_ID}/primary" 200 >/dev/null
request POST "/api/master-data/materials/${MATERIAL_ID}/suppliers/${SUPPLIER_ID}/primary" 200 >/dev/null

echo "7. create customer"
request POST "/api/master-data/customers" 200 "{
  \"customer_id\": \"${CUSTOMER_ID}\",
  \"customer_name\": \"Smoke Customer\",
  \"contact_person\": \"Buyer\",
  \"phone\": \"13900000000\",
  \"email\": \"customer@example.com\",
  \"address\": \"Customer Address\",
  \"credit_limit\": \"100000.00\"
}" >/dev/null

echo "8. create BOM, component, activate, validate, tree, explosion preview"
request POST "/api/master-data/boms" 200 "{
  \"bom_id\": \"${BOM_ID}\",
  \"bom_name\": \"Smoke BOM\",
  \"parent_material_id\": \"${FINISHED_ID}\",
  \"variant_code\": null,
  \"version\": \"1.0\",
  \"base_quantity\": \"1.00\",
  \"valid_from\": \"2026-05-01\",
  \"valid_to\": null,
  \"status\": \"草稿\",
  \"notes\": \"Phase 3 smoke BOM\"
}" >/dev/null

request POST "/api/master-data/boms/${BOM_ID}/components" 200 "{
  \"bom_id\": \"${BOM_ID}\",
  \"parent_material_id\": \"${FINISHED_ID}\",
  \"component_material_id\": \"${MATERIAL_ID}\",
  \"quantity\": \"2.00\",
  \"unit\": \"PCS\",
  \"bom_level\": 1,
  \"scrap_rate\": \"0.00\",
  \"is_critical\": true
}" >/dev/null

request POST "/api/master-data/boms/${BOM_ID}/activate" 200 >/dev/null
request POST "/api/master-data/boms/${BOM_ID}/validate" 200 >/dev/null
request GET "/api/master-data/boms/${BOM_ID}/tree" 200 >/dev/null
request POST "/api/master-data/boms/${BOM_ID}/explode-preview" 200 '{
  "quantity": 5
}' >/dev/null

echo "9. copy BOM"
request POST "/api/master-data/boms/${BOM_ID}/copy" 200 "{
  \"target_bom_id\": \"${COPIED_BOM_ID}\",
  \"bom_name\": \"Smoke BOM Copy\",
  \"parent_material_id\": \"${FINISHED_ID}\",
  \"variant_code\": null,
  \"version\": \"1.1\",
  \"base_quantity\": \"1.00\",
  \"valid_from\": \"2026-05-01\",
  \"valid_to\": null,
  \"notes\": \"Phase 3 smoke copied BOM\"
}" | jq -e '.data.header.bom_id == "'"${COPIED_BOM_ID}"'" and (.data.components | length == 1)' >/dev/null
request POST "/api/master-data/boms/${COPIED_BOM_ID}/validate" 200 >/dev/null

echo "10. create product variant"
request POST "/api/master-data/product-variants" 200 "{
  \"variant_code\": \"${VARIANT_CODE}\",
  \"variant_name\": \"Smoke Variant\",
  \"base_material_id\": \"${FINISHED_ID}\",
  \"bom_id\": \"${BOM_ID}\",
  \"standard_cost\": \"120.00\"
}" >/dev/null

echo "11. create work center and quality master data"
request POST "/api/master-data/work-centers" 200 "{
  \"work_center_id\": \"${WORK_CENTER_ID}\",
  \"work_center_name\": \"Smoke Work Center\",
  \"location\": \"Workshop A\",
  \"capacity_per_day\": 1000,
  \"efficiency\": \"100.00\"
}" >/dev/null

request POST "/api/master-data/inspection-chars" 200 "{
  \"char_id\": \"${CHAR_ID}\",
  \"char_name\": \"Smoke Dimension\",
  \"material_type\": \"原材料\",
  \"inspection_type\": \"来料检验\",
  \"method\": \"Caliper\",
  \"standard\": \"10 plus/minus 0.2\",
  \"unit\": \"mm\",
  \"lower_limit\": \"9.80\",
  \"upper_limit\": \"10.20\",
  \"is_critical\": true
}" >/dev/null

request POST "/api/master-data/defect-codes" 200 "{
  \"defect_code\": \"${DEFECT_CODE}\",
  \"defect_name\": \"Smoke Scratch\",
  \"category\": \"外观\",
  \"severity\": \"一般\",
  \"description\": \"Smoke defect code\"
}" >/dev/null

echo "12. list major master data"
request GET "/api/master-data/materials" 200 >/dev/null
request GET "/api/master-data/bins" 200 >/dev/null
request GET "/api/master-data/bins/${RAW_BIN}/capacity-utilization" 200 >/dev/null
request GET "/api/master-data/suppliers" 200 >/dev/null
request GET "/api/master-data/customers" 200 >/dev/null
request GET "/api/master-data/boms" 200 >/dev/null
request GET "/api/master-data/product-variants" 200 >/dev/null
request GET "/api/master-data/work-centers" 200 >/dev/null
request GET "/api/master-data/inspection-chars" 200 >/dev/null
request GET "/api/master-data/defect-codes" 200 >/dev/null

cat <<EOF
== Phase 3 smoke test completed ==
MATERIAL_ID=${MATERIAL_ID}
FINISHED_ID=${FINISHED_ID}
SUPPLIER_ID=${SUPPLIER_ID}
CUSTOMER_ID=${CUSTOMER_ID}
BOM_ID=${BOM_ID}
COPIED_BOM_ID=${COPIED_BOM_ID}
VARIANT_CODE=${VARIANT_CODE}
WORK_CENTER_ID=${WORK_CENTER_ID}
CHAR_ID=${CHAR_ID}
DEFECT_CODE=${DEFECT_CODE}
EOF
