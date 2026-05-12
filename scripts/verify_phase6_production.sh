#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${IMS_BASE_URL:-${BASE_URL:-http://localhost:8080}}"
TOKEN="${IMS_AUTH_TOKEN:-${TOKEN:-}}"

VARIANT_CODE="${PHASE6_VARIANT_CODE:-FIN-A001}"
FINISHED_MATERIAL_ID="${PHASE6_FINISHED_MATERIAL_ID:-FIN001}"
BOM_ID="${PHASE6_BOM_ID:-BOM-FIN-A01}"
WORK_CENTER_ID="${PHASE6_WORK_CENTER_ID:-WC-FIN}"
FINISHED_TO_BIN="${PHASE6_FINISHED_TO_BIN:-FG-D02}"
PLANNED_QTY="${PHASE6_PLANNED_QTY:-10}"
COMPONENT_STOCK_QTY="${PHASE6_COMPONENT_STOCK_QTY:-20}"
RUN_ID="${PHASE6_RUN_ID:-$(date +%Y%m%d%H%M%S)}"
FINISHED_BATCH_NUMBER="${PHASE6_FINISHED_BATCH_NUMBER:-P6FIN${RUN_ID:2}${RANDOM}}"

COMPONENT_MATERIALS=("TP001" "LCM001" "PROT001")
COMPONENT_BINS=("SF-C02" "RM-B01" "RM-A02")
COMPONENT_BATCHES=("B-TP-20260418" "B-LCM-20260403" "B-PROT-20260401")
COMPONENT_UNIT_PRICES=("49.0" "52.0" "4.5")

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "required command not found: $1"
  fi
}

make_acceptance_token() {
  python3 <<'PY'
import base64
import hashlib
import hmac
import json
import os
import pathlib
import time

dotenv = pathlib.Path(".env")
if dotenv.exists():
    for line in dotenv.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key in {"IMS_JWT_SECRET", "JWT_ISSUER", "JWT_EXPIRES_SECONDS"}:
            os.environ.setdefault(key, value.strip().strip("\"'"))

secret = os.environ.get("IMS_JWT_SECRET", "change-me-in-production").encode("utf-8")
issuer = os.environ.get("JWT_ISSUER", "cuba-ims")
expires = int(os.environ.get("JWT_EXPIRES_SECONDS", "86400"))
now = int(time.time())

permissions = [
    "bom:explode",
    "production:read",
    "production:write",
    "production:release",
    "production:complete",
    "production:variance-read",
    "batch:trace",
    "inventory:read",
    "inventory:post",
    "report:read",
    "report:refresh",
]

header = {"alg": "HS256", "typ": "JWT"}
claims = {
    "sub": os.environ.get("PHASE6_JWT_SUB", "00000000-0000-0000-0000-000000000006"),
    "username": os.environ.get("PHASE6_JWT_USERNAME", "phase6_acceptance"),
    "roles": ["PHASE6_ACCEPTANCE"],
    "permissions": permissions,
    "token_type": "access",
    "exp": now + expires,
    "iat": now,
    "iss": issuer,
}

def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")

signing_input = ".".join(
    b64url(json.dumps(part, separators=(",", ":"), sort_keys=True).encode("utf-8"))
    for part in (header, claims)
)
signature = hmac.new(secret, signing_input.encode("ascii"), hashlib.sha256).digest()
print(f"{signing_input}.{b64url(signature)}")
PY
}

request() {
  local method="$1"
  local path="$2"
  local expected="${3:-200}"
  local body="${4:-}"
  local response status payload
  local args=(-sS -w $'\n%{http_code}' -X "$method" "${BASE_URL}${path}" -H "Authorization: Bearer ${TOKEN}")

  if [[ -n "$body" ]]; then
    args+=(-H "Content-Type: application/json" -d "$body")
  fi

  if ! response="$(curl "${args[@]}")"; then
    fail "curl failed for ${method} ${path}"
  fi

  status="$(printf '%s' "$response" | tail -n 1)"
  payload="$(printf '%s' "$response" | sed '$d')"

  if [[ "$status" != "$expected" ]]; then
    echo "Unexpected status for ${method} ${path}: got ${status}, expected ${expected}" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  if [[ "$expected" == "200" ]]; then
    if ! printf '%s\n' "$payload" | jq -e '.success == true' >/dev/null; then
      echo "Expected success=true for ${method} ${path}" >&2
      printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
      exit 1
    fi
  fi

  printf '%s\n' "$payload"
}

request_no_auth() {
  local method="$1"
  local path="$2"
  local expected="${3:-200}"
  local response status payload

  if ! response="$(curl -sS -w $'\n%{http_code}' -X "$method" "${BASE_URL}${path}")"; then
    fail "curl failed for ${method} ${path}"
  fi

  status="$(printf '%s' "$response" | tail -n 1)"
  payload="$(printf '%s' "$response" | sed '$d')"

  if [[ "$status" != "$expected" ]]; then
    echo "Unexpected status for ${method} ${path}: got ${status}, expected ${expected}" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  printf '%s\n' "$payload"
}

assert_json() {
  local payload="$1"
  local message="$2"
  shift 2

  if ! printf '%s\n' "$payload" | jq -e "$@" >/dev/null; then
    echo "Assertion failed: ${message}" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi
}

sum_inventory_qty() {
  printf '%s\n' "$1" | jq -re '[((.data.items // .data // [])[]?.qty) | tonumber] | add // 0'
}

material_stock() {
  local material_id="$1"
  local response
  response="$(request GET "/api/inventory/current?material_id=${material_id}&page_size=200" 200)"
  sum_inventory_qty "$response"
}

post_component_stock() {
  local index="$1"
  local qty="$2"
  local material_id="${COMPONENT_MATERIALS[$index]}"
  local bin_code="${COMPONENT_BINS[$index]}"
  local batch_number="${COMPONENT_BATCHES[$index]}"
  local unit_price="${COMPONENT_UNIT_PRICES[$index]}"
  local body

  body="$(jq -n \
    --arg material_id "$material_id" \
    --arg batch_number "$batch_number" \
    --arg bin_code "$bin_code" \
    --arg run_id "$RUN_ID" \
    --argjson quantity "$qty" \
    --argjson unit_price "$unit_price" \
    '{
      material_id: $material_id,
      movement_type: "101",
      quantity: $quantity,
      to_bin: $bin_code,
      batch_number: $batch_number,
      reference_doc: ("P6-STOCK-" + $run_id),
      quality_status: "合格",
      unit_price: $unit_price,
      remark: ("phase 6 component stock " + $run_id)
    }')"

  request POST "/api/inventory/post" 200 "$body" >/dev/null
  echo "seeded ${material_id}/${bin_code}/${batch_number} +${qty}"
}

need curl
need jq

if [[ -z "$TOKEN" ]]; then
  need python3
  TOKEN="$(make_acceptance_token)"
  echo "Auth: generated local Phase 6 acceptance JWT"
else
  echo "Auth: using provided token"
fi

echo "== Phase 6 Acceptance Test =="
echo "BASE_URL=${BASE_URL}"
echo "VARIANT_CODE=${VARIANT_CODE}"
echo "FINISHED_MATERIAL_ID=${FINISHED_MATERIAL_ID}"
echo "BOM_ID=${BOM_ID}"
echo "WORK_CENTER_ID=${WORK_CENTER_ID}"
echo "FINISHED_TO_BIN=${FINISHED_TO_BIN}"
echo "FINISHED_BATCH_NUMBER=${FINISHED_BATCH_NUMBER}"
echo

echo "0. Health check"
request_no_auth GET "/health" 200 >/dev/null

echo "1. Seed component stock for repeatable completion"
for i in "${!COMPONENT_MATERIALS[@]}"; do
  current_qty="$(material_stock "${COMPONENT_MATERIALS[$i]}")"
  if [[ "$current_qty" -lt "$PLANNED_QTY" ]]; then
    seed_qty=$((PLANNED_QTY - current_qty))
    post_component_stock "$i" "$seed_qty"
  else
    echo "component ${COMPONENT_MATERIALS[$i]} already has ${current_qty}"
  fi
done

echo "2. Capture baseline finished stock"
BASELINE_RESPONSE="$(request GET "/api/inventory/current?material_id=${FINISHED_MATERIAL_ID}&bin_code=${FINISHED_TO_BIN}&page_size=200" 200)"
BASELINE_QTY="$(sum_inventory_qty "$BASELINE_RESPONSE")"
echo "BASELINE_FINISHED_QTY=${BASELINE_QTY}"

echo "3. Preview BOM explosion"
BOM_BODY="$(jq -n \
  --arg variant_code "$VARIANT_CODE" \
  --arg finished_material_id "$FINISHED_MATERIAL_ID" \
  --argjson quantity "$PLANNED_QTY" \
  '{
    variant_code: $variant_code,
    finished_material_id: $finished_material_id,
    quantity: $quantity,
    merge_components: true
  }')"
BOM_RESPONSE="$(request POST "/api/production/bom-explosion" 200 "$BOM_BODY")"
assert_json "$BOM_RESPONSE" \
  "BOM explosion should include component requirements" \
  --argjson planned_qty "$PLANNED_QTY" \
  '.data.components | length >= 3 and all(.[]; .required_qty >= $planned_qty)'

echo "4. Create production order"
CREATE_BODY="$(jq -n \
  --arg variant_code "$VARIANT_CODE" \
  --arg finished_material_id "$FINISHED_MATERIAL_ID" \
  --arg bom_id "$BOM_ID" \
  --arg work_center_id "$WORK_CENTER_ID" \
  --arg run_id "$RUN_ID" \
  --argjson planned_qty "$PLANNED_QTY" \
  '{
    variant_code: $variant_code,
    finished_material_id: $finished_material_id,
    bom_id: $bom_id,
    planned_qty: $planned_qty,
    work_center_id: $work_center_id,
    planned_start_date: "2026-05-05",
    planned_end_date: "2026-05-08",
    remark: ("phase 6 production order " + $run_id)
  }')"
CREATE_RESPONSE="$(request POST "/api/production-orders" 200 "$CREATE_BODY")"
ORDER_ID="$(printf '%s\n' "$CREATE_RESPONSE" | jq -re '.data.order_id')"
assert_json "$CREATE_RESPONSE" "created order should be planned" '(.data.status == "PLANNED" or .data.status == "计划中")'
echo "ORDER_ID=${ORDER_ID}"

echo "5. Verify generated direct component lines"
COMPONENT_RESPONSE="$(request GET "/api/production-orders/${ORDER_ID}/components" 200)"
assert_json "$COMPONENT_RESPONSE" \
  "production order should have direct BOM component lines" \
  --argjson planned_qty "$PLANNED_QTY" \
  '.data | length == 3 and all(.[]; .required_qty == $planned_qty and .issued_qty == 0)'

echo "6. Release production order"
RELEASE_RESPONSE="$(request POST "/api/production-orders/${ORDER_ID}/release" 200 '{"remark":"phase 6 release"}')"
assert_json "$RELEASE_RESPONSE" "released order should be released" '(.data.status == "RELEASED" or .data.status == "已下达")'

echo "7. Complete production order"
COMPLETE_BODY="$(jq -n \
  --arg batch_number "$FINISHED_BATCH_NUMBER" \
  --arg finished_to_bin "$FINISHED_TO_BIN" \
  --arg run_id "$RUN_ID" \
  --argjson completed_qty "$PLANNED_QTY" \
  '{
    completed_qty: $completed_qty,
    finished_batch_number: $batch_number,
    finished_to_bin: $finished_to_bin,
    pick_strategy: "FEFO",
    remark: ("phase 6 complete " + $run_id)
  }')"
COMPLETE_RESPONSE="$(request POST "/api/production-orders/${ORDER_ID}/complete" 200 "$COMPLETE_BODY")"
assert_json "$COMPLETE_RESPONSE" \
  "completion should post component issues, finished receipt, genealogy, and variance marker" \
  --argjson completed_qty "$PLANNED_QTY" \
  '(.data.status == "COMPLETED" or .data.status == "完成" or .data.status == "Completed")
   and .data.completed_qty == $completed_qty
   and .data.finished_transaction.movement_type == "101"
   and .data.finished_transaction.quantity == $completed_qty
   and (.data.component_transactions | length) >= 3
   and ([.data.component_transactions[] | select(.movement_type == "261") | .quantity] | add) >= ($completed_qty * 3)
   and .data.genealogy_count >= 3
   and (.data.variance_id | type == "string")
   and .data.reports_stale == true'

echo "8. Query genealogy and variance"
GENEALOGY_RESPONSE="$(request GET "/api/production-orders/${ORDER_ID}/genealogy" 200)"
assert_json "$GENEALOGY_RESPONSE" \
  "genealogy should contain finished batch and components" \
  --arg batch_number "$FINISHED_BATCH_NUMBER" \
  '.data | length >= 3 and all(.[]; .parent_batch_number == $batch_number)'

VARIANCE_RESPONSE="$(request GET "/api/production-orders/${ORDER_ID}/variance" 200)"
assert_json "$VARIANCE_RESPONSE" \
  "variance should include planned and actual material cost" \
  '.data.planned_material_cost != null and .data.actual_material_cost != null and .data.material_variance != null and .data.total_variance != null'

echo "9. Verify finished stock delta"
FINAL_RESPONSE="$(request GET "/api/inventory/current?material_id=${FINISHED_MATERIAL_ID}&bin_code=${FINISHED_TO_BIN}&batch_number=${FINISHED_BATCH_NUMBER}&page_size=200" 200)"
FINAL_BATCH_QTY="$(sum_inventory_qty "$FINAL_RESPONSE")"

if [[ "$FINAL_BATCH_QTY" -ne "$PLANNED_QTY" ]]; then
  echo "Finished batch stock mismatch: got ${FINAL_BATCH_QTY}, expected ${PLANNED_QTY}" >&2
  printf '%s\n' "$FINAL_RESPONSE" | jq .
  exit 1
fi

request POST "/api/reports/refresh" 200 >/dev/null

echo "FINAL_BATCH_QTY=${FINAL_BATCH_QTY}"
echo "EXPECTED_FINISHED_DELTA=${PLANNED_QTY}"
echo
echo "== Phase 6 acceptance passed =="
