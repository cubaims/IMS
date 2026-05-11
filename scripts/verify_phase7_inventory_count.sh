#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${IMS_BASE_URL:-${BASE_URL:-http://localhost:8080}}"
TOKEN="${IMS_AUTH_TOKEN:-${TOKEN:-}}"
DB_URL="${IMS_DATABASE_URL:-${DATABASE_URL:-}}"

COUNT_BIN="${PHASE7_COUNT_BIN:-RM-A02}"
GAIN_MATERIAL="${PHASE7_GAIN_MATERIAL:-FPC001}"
GAIN_BATCH="${PHASE7_GAIN_BATCH:-B-FPC-20260402}"
LOSS_MATERIAL="${PHASE7_LOSS_MATERIAL:-PROT001}"
LOSS_BATCH="${PHASE7_LOSS_BATCH:-B-PROT-20260401}"
RUN_ID="${PHASE7_RUN_ID:-$(date +%Y%m%d%H%M%S)}"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "required command not found: $1"
  fi
}

load_dotenv_database_url() {
  if [[ -n "$DB_URL" || ! -f .env ]]; then
    return
  fi

  DB_URL="$(
    awk -F= '$1 == "DATABASE_URL" { print substr($0, index($0, "=") + 1) }' .env \
      | tail -n 1 \
      | sed -e 's/^["'\'']//; s/["'\'']$//'
  )"
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
    "inventory:read",
    "inventory:post",
    "inventory:history",
    "inventory-count:read",
    "inventory-count:write",
    "inventory-count:submit",
    "inventory-count:approve",
    "inventory-count:post",
    "inventory-count:close",
    "report:read",
    "report:refresh",
]

header = {"alg": "HS256", "typ": "JWT"}
claims = {
    "sub": os.environ.get("PHASE7_JWT_SUB", "00000000-0000-0000-0000-000000000007"),
    "username": os.environ.get("PHASE7_JWT_USERNAME", "phase7_acceptance"),
    "roles": ["PHASE7_ACCEPTANCE"],
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

request_error() {
  local method="$1"
  local path="$2"
  local expected_status="$3"
  local expected_code="$4"
  local body="${5:-}"
  local payload

  payload="$(request "$method" "$path" "$expected_status" "$body")"
  assert_json "$payload" "error response should be ${expected_code}" \
    --arg code "$expected_code" \
    '.success == false and .error_code == $code'
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

stock_qty() {
  local material="$1"
  local batch="$2"
  local response
  response="$(request GET "/api/inventory/current?material_id=${material}&bin_code=${COUNT_BIN}&batch_number=${batch}&page_size=200" 200)"
  printf '%s\n' "$response" | jq -r '[.data[]?.qty | tonumber] | add // 0'
}

post_seed_stock() {
  local material="$1"
  local batch="$2"
  local quantity="$3"
  local reference="P7-SEED-${RUN_ID}"
  local body

  body="$(jq -n \
    --arg material_id "$material" \
    --arg batch_number "$batch" \
    --arg bin_code "$COUNT_BIN" \
    --arg reference_doc "$reference" \
    --argjson quantity "$quantity" \
    '{
      material_id: $material_id,
      movement_type: "101",
      quantity: $quantity,
      to_bin: $bin_code,
      batch_number: $batch_number,
      reference_doc: $reference_doc,
      quality_status: "合格",
      unit_price: 1,
      remark: "phase 7 acceptance seed stock"
    }')"

  request POST "/api/inventory/post" 200 "$body" >/dev/null
  echo "seeded ${material}/${COUNT_BIN}/${batch} +${quantity}"
}

ensure_stock() {
  local material="$1"
  local batch="$2"
  local minimum="$3"
  local current needed

  current="$(stock_qty "$material" "$batch")"
  if jq -en --argjson current "$current" --argjson minimum "$minimum" '$current >= $minimum' >/dev/null; then
    echo "stock ok ${material}/${COUNT_BIN}/${batch}=${current}"
    return
  fi

  needed="$(jq -n --argjson current "$current" --argjson minimum "$minimum" '$minimum - $current')"
  post_seed_stock "$material" "$batch" "$needed"
}

cleanup_open_counts() {
  local response
  response="$(request GET "/api/inventory/counts?count_scope=BIN&bin_code=${COUNT_BIN}&page_size=200" 200)"

  printf '%s\n' "$response" | jq -r '
    .data.items[]?
    | select(.status != "CLOSED" and .status != "CANCELLED")
    | [.count_doc_id, .status] | @tsv
  ' | while IFS=$'\t' read -r count_doc_id status; do
    [[ -n "$count_doc_id" ]] || continue
    case "$status" in
      POSTED)
        request POST "/api/inventory/counts/${count_doc_id}/close" 200 '{"remark":"phase 7 cleanup posted count"}' >/dev/null
        echo "closed stale posted count ${count_doc_id}"
        ;;
      *)
        request POST "/api/inventory/counts/${count_doc_id}/cancel" 200 '{"remark":"phase 7 cleanup open count"}' >/dev/null
        echo "cancelled stale count ${count_doc_id}"
        ;;
    esac
  done
}

db_query_for_count() {
  local count_doc_id="$1"
  local sql="$2"
  printf '%s\n' "$sql" \
    | psql "$DB_URL" -AtX -v ON_ERROR_STOP=1 -v count_doc_id="$count_doc_id" -f -
}

verify_db_posting_results() {
  local count_doc_id="$1"
  local expected_header_status="$2"
  local header_status movement_types missing_tx_id tx_count linked_count

  [[ -n "$DB_URL" ]] || return

  header_status="$(db_query_for_count "$count_doc_id" \
    "SELECT status FROM wms.wms_inventory_count_h WHERE count_doc_id = :'count_doc_id'")"
  [[ "$header_status" == "$expected_header_status" ]] \
    || fail "DB header status mismatch for ${count_doc_id}: got ${header_status}, expected ${expected_header_status}"

  movement_types="$(db_query_for_count "$count_doc_id" \
    "SELECT COALESCE(string_agg(movement_type::text, ',' ORDER BY movement_type::text), '')
       FROM wms.wms_inventory_count_d
      WHERE count_doc_id = :'count_doc_id'
        AND COALESCE(difference_qty, 0) <> 0")"
  [[ "$movement_types" == "701,702" ]] \
    || fail "DB movement types mismatch for ${count_doc_id}: ${movement_types}"

  missing_tx_id="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_inventory_count_d
      WHERE count_doc_id = :'count_doc_id'
        AND COALESCE(difference_qty, 0) <> 0
        AND transaction_id IS NULL")"
  [[ "$missing_tx_id" == "0" ]] \
    || fail "DB has difference lines without transaction_id for ${count_doc_id}"

  tx_count="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_transactions
      WHERE reference_doc = :'count_doc_id'
        AND movement_type::text IN ('701', '702')")"
  [[ "$tx_count" == "2" ]] \
    || fail "DB transaction count mismatch for ${count_doc_id}: ${tx_count}"

  linked_count="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_inventory_count_d d
       JOIN wms.wms_transactions t ON t.transaction_id = d.transaction_id
      WHERE d.count_doc_id = :'count_doc_id'
        AND d.movement_type::text IN ('701', '702')
        AND t.reference_doc = :'count_doc_id'")"
  [[ "$linked_count" == "2" ]] \
    || fail "DB transaction_id back references mismatch for ${count_doc_id}: ${linked_count}"

  echo "DB_POSTING_OK=${count_doc_id} status=${header_status} movement_types=${movement_types}"
}

verify_db_no_half_posting() {
  local count_doc_id="$1"
  local header_status tx_count line_tx_count posted_line_count

  [[ -n "$DB_URL" ]] || return

  header_status="$(db_query_for_count "$count_doc_id" \
    "SELECT status FROM wms.wms_inventory_count_h WHERE count_doc_id = :'count_doc_id'")"
  [[ "$header_status" == "APPROVED" ]] \
    || fail "rollback header status mismatch for ${count_doc_id}: ${header_status}"

  tx_count="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_transactions
      WHERE reference_doc = :'count_doc_id'
        AND movement_type::text IN ('701', '702')")"
  [[ "$tx_count" == "0" ]] \
    || fail "rollback left inventory transactions for ${count_doc_id}: ${tx_count}"

  line_tx_count="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_inventory_count_d
      WHERE count_doc_id = :'count_doc_id'
        AND transaction_id IS NOT NULL")"
  [[ "$line_tx_count" == "0" ]] \
    || fail "rollback left transaction_id on lines for ${count_doc_id}: ${line_tx_count}"

  posted_line_count="$(db_query_for_count "$count_doc_id" \
    "SELECT COUNT(*)
       FROM wms.wms_inventory_count_d
      WHERE count_doc_id = :'count_doc_id'
        AND status = 'POSTED'")"
  [[ "$posted_line_count" == "0" ]] \
    || fail "rollback left POSTED lines for ${count_doc_id}: ${posted_line_count}"

  echo "DB_ROLLBACK_OK=${count_doc_id}"
}

post_issue_stock() {
  local material="$1"
  local batch="$2"
  local quantity="$3"
  local reference="$4"
  local body

  body="$(jq -n \
    --arg material_id "$material" \
    --arg batch_number "$batch" \
    --arg bin_code "$COUNT_BIN" \
    --arg reference_doc "$reference" \
    --argjson quantity "$quantity" \
    '{
      material_id: $material_id,
      movement_type: "261",
      quantity: $quantity,
      from_bin: $bin_code,
      batch_number: $batch_number,
      reference_doc: $reference_doc,
      quality_status: "合格",
      remark: "phase 7 rollback drain stock"
    }')"

  request POST "/api/inventory/post" 200 "$body" >/dev/null
  echo "drained ${material}/${COUNT_BIN}/${batch} -${quantity}"
}

verify_post_failure_rollback() {
  local rollback_create_body rollback_create_response rollback_generate_response
  local rollback_count_doc_id line_payload line_no body drain_qty posting_date post_body failure_payload

  cleanup_open_counts
  ensure_stock "$GAIN_MATERIAL" "$GAIN_BATCH" 3
  ensure_stock "$LOSS_MATERIAL" "$LOSS_BATCH" 3

  rollback_create_body="$(jq -n \
    --arg bin_code "$COUNT_BIN" \
    --arg run_id "$RUN_ID" \
    '{
      count_type: "CYCLE",
      count_scope: "BIN",
      bin_code: $bin_code,
      remark: ("phase 7 rollback " + $run_id)
    }')"

  rollback_create_response="$(request POST "/api/inventory/counts" 200 "$rollback_create_body")"
  rollback_count_doc_id="$(printf '%s\n' "$rollback_create_response" | jq -r '.data.count_doc_id')"
  echo "ROLLBACK_COUNT_DOC_ID=${rollback_count_doc_id}"

  rollback_generate_response="$(request POST "/api/inventory/counts/${rollback_count_doc_id}/generate-lines" 200)"

  printf '%s\n' "$rollback_generate_response" | jq -c \
    --arg gain_material "$GAIN_MATERIAL" \
    --arg gain_batch "$GAIN_BATCH" \
    --arg loss_material "$LOSS_MATERIAL" \
    --arg loss_batch "$LOSS_BATCH" \
    '.data.lines[]
     | . as $line
     | ($line.system_qty | tonumber) as $system_qty
     | {
         line_no: $line.line_no,
         counted_qty:
           (if $line.material_id == $gain_material and $line.batch_number == $gain_batch then $system_qty + 1
            elif $line.material_id == $loss_material and $line.batch_number == $loss_batch then $system_qty - 1
            else $system_qty end),
         difference_reason:
           (if $line.material_id == $gain_material and $line.batch_number == $gain_batch then "phase 7 rollback gain"
            elif $line.material_id == $loss_material and $line.batch_number == $loss_batch then "phase 7 rollback loss"
            else null end),
         remark: "phase 7 rollback counted"
       }' | while read -r line_payload; do
    line_no="$(printf '%s\n' "$line_payload" | jq -r '.line_no')"
    body="$(printf '%s\n' "$line_payload" | jq '{counted_qty, difference_reason, remark}')"
    request PATCH "/api/inventory/counts/${rollback_count_doc_id}/lines/${line_no}" 200 "$body" >/dev/null
  done

  request POST "/api/inventory/counts/${rollback_count_doc_id}/submit" 200 '{"remark":"phase 7 rollback submit"}' >/dev/null
  request POST "/api/inventory/counts/${rollback_count_doc_id}/approve" 200 '{"approved":true,"remark":"phase 7 rollback approve"}' >/dev/null

  drain_qty="$(stock_qty "$LOSS_MATERIAL" "$LOSS_BATCH")"
  if ! jq -en --argjson drain_qty "$drain_qty" '$drain_qty > 0' >/dev/null; then
    fail "rollback setup needs positive loss stock, got ${drain_qty}"
  fi

  post_issue_stock "$LOSS_MATERIAL" "$LOSS_BATCH" "$drain_qty" "P7-DRAIN-${RUN_ID}"

  posting_date="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  post_body="$(jq -n --arg posting_date "$posting_date" '{posting_date: $posting_date, remark: "phase 7 rollback post"}')"
  failure_payload="$(request_error POST "/api/inventory/counts/${rollback_count_doc_id}/post" 500 "COUNT_DIFFERENCE_POST_FAILED" "$post_body")"
  assert_json "$failure_payload" "post failure should be structured" '.error_code == "COUNT_DIFFERENCE_POST_FAILED"'

  post_seed_stock "$LOSS_MATERIAL" "$LOSS_BATCH" "$drain_qty"
  verify_db_no_half_posting "$rollback_count_doc_id"

  request POST "/api/inventory/counts/${rollback_count_doc_id}/cancel" 200 '{"remark":"phase 7 rollback cleanup"}' >/dev/null
}

need curl
need jq
load_dotenv_database_url
if [[ -n "$DB_URL" ]]; then
  need psql
  echo "DB: direct PostgreSQL assertions enabled"
else
  echo "DB: DATABASE_URL not set, direct PostgreSQL assertions skipped"
fi

if [[ -z "$TOKEN" ]]; then
  need python3
  TOKEN="$(make_acceptance_token)"
  echo "Auth: generated local Phase 7 acceptance JWT"
else
  echo "Auth: using provided token"
fi

echo "== Phase 7 Acceptance Test =="
request GET "/health" 200 >/dev/null
request_error GET "/api/inventory/counts/COUNT-NOT-FOUND-${RUN_ID}" 404 "INVENTORY_COUNT_NOT_FOUND" >/dev/null

ensure_stock "$GAIN_MATERIAL" "$GAIN_BATCH" 3
ensure_stock "$LOSS_MATERIAL" "$LOSS_BATCH" 3
cleanup_open_counts

GAIN_BEFORE="$(stock_qty "$GAIN_MATERIAL" "$GAIN_BATCH")"
LOSS_BEFORE="$(stock_qty "$LOSS_MATERIAL" "$LOSS_BATCH")"

create_body="$(jq -n \
  --arg bin_code "$COUNT_BIN" \
  --arg run_id "$RUN_ID" \
  '{
    count_type: "CYCLE",
    count_scope: "BIN",
    bin_code: $bin_code,
    remark: ("phase 7 acceptance " + $run_id)
  }')"

create_response="$(request POST "/api/inventory/counts" 200 "$create_body")"
COUNT_DOC_ID="$(printf '%s\n' "$create_response" | jq -r '.data.count_doc_id')"
assert_json "$create_response" "created count should be DRAFT" '.data.status == "DRAFT"'
echo "COUNT_DOC_ID=${COUNT_DOC_ID}"

request_error POST "/api/inventory/counts" 409 "INVENTORY_COUNT_DUPLICATED_SCOPE" "$create_body" >/dev/null

generate_response="$(request POST "/api/inventory/counts/${COUNT_DOC_ID}/generate-lines" 200)"
assert_json "$generate_response" "generated count should be COUNTING" '.data.status == "COUNTING"'
assert_json "$generate_response" "gain/loss seed lines should exist" \
  --arg gain_material "$GAIN_MATERIAL" \
  --arg gain_batch "$GAIN_BATCH" \
  --arg loss_material "$LOSS_MATERIAL" \
  --arg loss_batch "$LOSS_BATCH" \
  '([.data.lines[] | select(.material_id == $gain_material and .batch_number == $gain_batch)] | length) == 1
   and ([.data.lines[] | select(.material_id == $loss_material and .batch_number == $loss_batch)] | length) == 1'

request_error POST "/api/inventory/counts/${COUNT_DOC_ID}/generate-lines" 409 "INVENTORY_COUNT_STATUS_INVALID" >/dev/null

first_line_no="$(printf '%s\n' "$generate_response" | jq -r '.data.lines[0].line_no')"
request_error PATCH "/api/inventory/counts/${COUNT_DOC_ID}/lines/${first_line_no}" 400 "COUNTED_QTY_INVALID" \
  '{"counted_qty":1.5,"difference_reason":"invalid fractional count","remark":"phase 7 invalid counted qty"}' >/dev/null

printf '%s\n' "$generate_response" | jq -c \
  --arg gain_material "$GAIN_MATERIAL" \
  --arg gain_batch "$GAIN_BATCH" \
  --arg loss_material "$LOSS_MATERIAL" \
  --arg loss_batch "$LOSS_BATCH" \
  '.data.lines[]
   | . as $line
   | ($line.system_qty | tonumber) as $system_qty
   | {
       line_no: $line.line_no,
       counted_qty:
         (if $line.material_id == $gain_material and $line.batch_number == $gain_batch then $system_qty + 1
          elif $line.material_id == $loss_material and $line.batch_number == $loss_batch then $system_qty - 1
          else $system_qty end),
       difference_reason:
         (if $line.material_id == $gain_material and $line.batch_number == $gain_batch then "phase 7 gain"
          elif $line.material_id == $loss_material and $line.batch_number == $loss_batch then "phase 7 loss"
          else null end),
       remark: "phase 7 acceptance counted"
     }' | while read -r line_payload; do
  line_no="$(printf '%s\n' "$line_payload" | jq -r '.line_no')"
  body="$(printf '%s\n' "$line_payload" | jq '{counted_qty, difference_reason, remark}')"
  request PATCH "/api/inventory/counts/${COUNT_DOC_ID}/lines/${line_no}" 200 "$body" >/dev/null
done

detail_response="$(request GET "/api/inventory/counts/${COUNT_DOC_ID}" 200)"
assert_json "$detail_response" "gain line should be 701" \
  --arg material "$GAIN_MATERIAL" --arg batch "$GAIN_BATCH" \
  '.data.lines[] | select(.material_id == $material and .batch_number == $batch) | .movement_type == "701"'
assert_json "$detail_response" "loss line should be 702" \
  --arg material "$LOSS_MATERIAL" --arg batch "$LOSS_BATCH" \
  '.data.lines[] | select(.material_id == $material and .batch_number == $batch) | .movement_type == "702"'

request POST "/api/inventory/counts/${COUNT_DOC_ID}/submit" 200 '{"remark":"phase 7 submit"}' >/dev/null
request POST "/api/inventory/counts/${COUNT_DOC_ID}/approve" 200 '{"approved":true,"remark":"phase 7 approve"}' >/dev/null

posting_date="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
post_body="$(jq -n --arg posting_date "$posting_date" '{posting_date: $posting_date, remark: "phase 7 post"}')"
post_response="$(request POST "/api/inventory/counts/${COUNT_DOC_ID}/post" 200 "$post_body")"
assert_json "$post_response" "count should be posted" '.data.status == "POSTED" and .data.reports_stale == true'
assert_json "$post_response" "posting should contain 701 and 702" \
  '([.data.transactions[].movement_type] | sort) == ["701","702"]'

request POST "/api/inventory/counts/${COUNT_DOC_ID}/close" 200 '{"remark":"phase 7 close"}' >/dev/null
closed_response="$(request GET "/api/inventory/counts/${COUNT_DOC_ID}" 200)"
assert_json "$closed_response" "count should be CLOSED" '.data.status == "CLOSED"'
verify_db_posting_results "$COUNT_DOC_ID" "CLOSED"

tx_response="$(request GET "/api/inventory/transactions?reference_doc=${COUNT_DOC_ID}&page_size=20" 200)"
assert_json "$tx_response" "transaction history should include 701 and 702" \
  '([.data[]?.movement_type | tostring] | sort) == ["701","702"]'

GAIN_AFTER="$(stock_qty "$GAIN_MATERIAL" "$GAIN_BATCH")"
LOSS_AFTER="$(stock_qty "$LOSS_MATERIAL" "$LOSS_BATCH")"

jq -en \
  --argjson before "$GAIN_BEFORE" \
  --argjson after "$GAIN_AFTER" \
  '$after == ($before + 1)' >/dev/null || fail "gain stock delta mismatch: before=${GAIN_BEFORE}, after=${GAIN_AFTER}"

jq -en \
  --argjson before "$LOSS_BEFORE" \
  --argjson after "$LOSS_AFTER" \
  '$after == ($before - 1)' >/dev/null || fail "loss stock delta mismatch: before=${LOSS_BEFORE}, after=${LOSS_AFTER}"

echo "GAIN_STOCK_DELTA=+1 (${GAIN_BEFORE} -> ${GAIN_AFTER})"
echo "LOSS_STOCK_DELTA=-1 (${LOSS_BEFORE} -> ${LOSS_AFTER})"
verify_post_failure_rollback
echo "== Phase 7 acceptance passed =="
