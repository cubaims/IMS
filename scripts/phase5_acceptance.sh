#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${IMS_BASE_URL:-${BASE_URL:-http://localhost:8080}}"
TOKEN="${IMS_AUTH_TOKEN:-${TOKEN:-}}"

SUPPLIER_ID="${PHASE5_SUPPLIER_ID:-SUP-A001}"
CUSTOMER_ID="${PHASE5_CUSTOMER_ID:-CUST-001}"
MATERIAL_ID="${PHASE5_MATERIAL_ID:-CG001}"
BIN_CODE="${PHASE5_BIN_CODE:-RM-A02}"
RECEIPT_QTY="${PHASE5_RECEIPT_QTY:-100}"
SHIPMENT_QTY="${PHASE5_SHIPMENT_QTY:-20}"
UNIT_PRICE="${PHASE5_UNIT_PRICE:-12.5}"
SALES_PRICE="${PHASE5_SALES_PRICE:-20.0}"
RUN_ID="${PHASE5_RUN_ID:-$(date +%Y%m%d%H%M%S)}"
BATCH_NUMBER="${PHASE5_BATCH_NUMBER:-P5CG${RUN_ID:2}${RANDOM}}"

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
    "purchase:read",
    "purchase:write",
    "purchase:receipt",
    "sales:read",
    "sales:write",
    "sales:shipment",
    "report:read",
    "report:refresh",
]

header = {"alg": "HS256", "typ": "JWT"}
claims = {
    "sub": os.environ.get("PHASE5_JWT_SUB", "00000000-0000-0000-0000-000000000005"),
    "username": os.environ.get("PHASE5_JWT_USERNAME", "phase5_acceptance"),
    "roles": ["PHASE5_ACCEPTANCE"],
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

sum_stock_qty() {
  printf '%s\n' "$1" | jq -re '[.data.items[]?.qty] | add // 0'
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

need curl
need jq

if [[ -z "$TOKEN" ]]; then
  need python3
  TOKEN="$(make_acceptance_token)"
  echo "Auth: generated local Phase 5 acceptance JWT"
else
  echo "Auth: using provided token"
fi

echo "== Phase 5 Acceptance Test =="
echo "BASE_URL=${BASE_URL}"
echo "SUPPLIER_ID=${SUPPLIER_ID}"
echo "CUSTOMER_ID=${CUSTOMER_ID}"
echo "MATERIAL_ID=${MATERIAL_ID}"
echo "BIN_CODE=${BIN_CODE}"
echo "BATCH_NUMBER=${BATCH_NUMBER}"
echo

echo "0. Health check"
request_no_auth GET "/health" 200 >/dev/null

echo "1. Refresh reports and capture baseline stock"
request POST "/api/reports/refresh" 200 >/dev/null
BASELINE_STOCK_RESPONSE="$(request GET "/api/reports/current-stock?material_id=${MATERIAL_ID}&bin_code=${BIN_CODE}&page_size=200" 200)"
BASELINE_QTY="$(sum_stock_qty "$BASELINE_STOCK_RESPONSE")"
echo "BASELINE_QTY=${BASELINE_QTY}"

echo "2. Create PO"
PO_BODY="$(jq -n \
  --arg supplier_id "$SUPPLIER_ID" \
  --arg material_id "$MATERIAL_ID" \
  --arg bin_code "$BIN_CODE" \
  --argjson ordered_qty "$RECEIPT_QTY" \
  --argjson unit_price "$UNIT_PRICE" \
  --arg run_id "$RUN_ID" \
  '{
    supplier_id: $supplier_id,
    expected_date: "2026-05-15",
    remark: ("phase 5 po acceptance " + $run_id),
    lines: [{
      line_no: 10,
      material_id: $material_id,
      ordered_qty: $ordered_qty,
      unit_price: $unit_price,
      expected_bin: $bin_code
    }]
  }')"
PO_RESPONSE="$(request POST "/api/purchase-orders" 200 "$PO_BODY")"
PO_ID="$(printf '%s\n' "$PO_RESPONSE" | jq -re '.data.po_id')"
assert_json "$PO_RESPONSE" "PO creation should return status" '.data.status | type == "string"'
echo "PO_ID=${PO_ID}"

echo "3. Post PO receipt"
RECEIPT_BODY="$(jq -n \
  --arg batch_number "$BATCH_NUMBER" \
  --arg bin_code "$BIN_CODE" \
  --arg run_id "$RUN_ID" \
  --argjson receipt_qty "$RECEIPT_QTY" \
  '{
    remark: ("phase 5 receipt acceptance " + $run_id),
    lines: [{
      line_no: 10,
      receipt_qty: $receipt_qty,
      batch_number: $batch_number,
      to_bin: $bin_code
    }]
  }')"
RECEIPT_RESPONSE="$(request POST "/api/purchase-orders/${PO_ID}/receipt" 200 "$RECEIPT_BODY")"
assert_json "$RECEIPT_RESPONSE" \
  "PO receipt should post one 101 transaction for receipt quantity" \
  --argjson receipt_qty "$RECEIPT_QTY" \
  '.data.transactions | length == 1 and .[0].movement_type == "101" and .[0].quantity == $receipt_qty' \
  >/dev/null

echo "4. Create SO"
SO_BODY="$(jq -n \
  --arg customer_id "$CUSTOMER_ID" \
  --arg material_id "$MATERIAL_ID" \
  --arg bin_code "$BIN_CODE" \
  --arg run_id "$RUN_ID" \
  --argjson ordered_qty "$SHIPMENT_QTY" \
  --argjson unit_price "$SALES_PRICE" \
  '{
    customer_id: $customer_id,
    required_date: "2026-05-20",
    remark: ("phase 5 so acceptance " + $run_id),
    lines: [{
      line_no: 10,
      material_id: $material_id,
      ordered_qty: $ordered_qty,
      unit_price: $unit_price,
      from_bin: $bin_code
    }]
  }')"
SO_RESPONSE="$(request POST "/api/sales-orders" 200 "$SO_BODY")"
SO_ID="$(printf '%s\n' "$SO_RESPONSE" | jq -re '.data.so_id')"
assert_json "$SO_RESPONSE" "SO creation should return status" '.data.status | type == "string"'
echo "SO_ID=${SO_ID}"

echo "5. Preview FEFO"
PICK_PREVIEW_BODY="$(jq -n \
  --argjson shipment_qty "$SHIPMENT_QTY" \
  '{lines: [{line_no: 10, shipment_qty: $shipment_qty}]}')"
PICK_PREVIEW_RESPONSE="$(request POST "/api/sales-orders/${SO_ID}/pick-preview" 200 "$PICK_PREVIEW_BODY")"
assert_json "$PICK_PREVIEW_RESPONSE" \
  "FEFO preview should allocate the full shipment quantity" \
  --argjson shipment_qty "$SHIPMENT_QTY" \
  '.data.lines | length == 1 and (.[0].picks | length > 0) and ((.[0].picks | map(.pick_qty) | add) == $shipment_qty)' \
  >/dev/null

echo "6. Post SO shipment"
SHIPMENT_BODY="$(jq -n \
  --arg run_id "$RUN_ID" \
  --argjson shipment_qty "$SHIPMENT_QTY" \
  '{
    pick_strategy: "FEFO",
    remark: ("phase 5 shipment acceptance " + $run_id),
    lines: [{line_no: 10, shipment_qty: $shipment_qty}]
  }')"
SHIPMENT_RESPONSE="$(request POST "/api/sales-orders/${SO_ID}/shipment" 200 "$SHIPMENT_BODY")"
assert_json "$SHIPMENT_RESPONSE" \
  "SO shipment should post 261 transactions for shipment quantity" \
  --argjson shipment_qty "$SHIPMENT_QTY" \
  '.data.transactions | length > 0 and (map(select(.movement_type == "261").quantity) | add == $shipment_qty)' \
  >/dev/null

echo "7. Refresh reports and verify current stock"
REFRESH_RESPONSE="$(request POST "/api/reports/refresh" 200)"
assert_json "$REFRESH_RESPONSE" "report refresh should succeed" '.data.refreshed == true'

FINAL_STOCK_RESPONSE="$(request GET "/api/reports/current-stock?material_id=${MATERIAL_ID}&bin_code=${BIN_CODE}&page_size=200" 200)"
FINAL_QTY="$(sum_stock_qty "$FINAL_STOCK_RESPONSE")"
EXPECTED_FINAL_QTY=$((BASELINE_QTY + RECEIPT_QTY - SHIPMENT_QTY))

if [[ "$FINAL_QTY" -ne "$EXPECTED_FINAL_QTY" ]]; then
  echo "Current stock mismatch for ${MATERIAL_ID}/${BIN_CODE}: got ${FINAL_QTY}, expected ${EXPECTED_FINAL_QTY}" >&2
  printf '%s\n' "$FINAL_STOCK_RESPONSE" | jq .
  exit 1
fi

echo "FINAL_QTY=${FINAL_QTY}"
echo "EXPECTED_DELTA=$((RECEIPT_QTY - SHIPMENT_QTY))"
echo
echo "== Phase 5 acceptance passed =="
