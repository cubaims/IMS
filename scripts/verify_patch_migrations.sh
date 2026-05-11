#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_URL="${IMS_DATABASE_URL:-${DATABASE_URL:-}}"
BASE_URL="${IMS_BASE_URL:-${BASE_URL:-http://localhost:8080}}"
DEMO_PASSWORD="${IMS_DEMO_PASSWORD:-password}"
VERIFY_PATCH_API="${VERIFY_PATCH_API:-0}"

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
  if [[ -n "$DB_URL" || ! -f "$PROJECT_ROOT/.env" ]]; then
    return
  fi

  DB_URL="$(
    awk -F= '$1 == "DATABASE_URL" { print substr($0, index($0, "=") + 1) }' "$PROJECT_ROOT/.env" \
      | tail -n 1 \
      | sed -e 's/^["'\'']//; s/["'\'']$//'
  )"
}

psql_scalar() {
  psql "$DB_URL" -X -v ON_ERROR_STOP=1 -Atq -c "$1"
}

expect_no_rows() {
  local label="$1"
  local sql="$2"
  local rows

  rows="$(psql_scalar "$sql")"
  if [[ -n "$rows" ]]; then
    echo "FAILED: $label" >&2
    printf '%s\n' "$rows" >&2
    exit 1
  fi

  echo "OK: $label"
}

login() {
  local username="$1"
  local response status payload body

  body="$(jq -n --arg username "$username" --arg password "$DEMO_PASSWORD" \
    '{username: $username, password: $password}')"

  if ! response="$(curl -sS -w $'\n%{http_code}' -X POST "${BASE_URL}/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "$body")"; then
    fail "curl failed for demo login: $username"
  fi

  status="$(printf '%s' "$response" | tail -n 1)"
  payload="$(printf '%s' "$response" | sed '$d')"

  if [[ "$status" != "200" ]]; then
    echo "FAILED: demo login $username returned HTTP $status" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  if ! printf '%s\n' "$payload" | jq -e --arg username "$username" \
    '.success == true and .data.access_token and .data.user.username == $username' >/dev/null; then
    echo "FAILED: demo login payload for $username" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  printf '%s\n' "$payload"
}

assert_permissions() {
  local payload="$1"
  shift

  local permission
  for permission in "$@"; do
    if ! printf '%s\n' "$payload" | jq -e --arg permission "$permission" \
      '.data.user.permissions | index($permission) != null' >/dev/null; then
      echo "FAILED: login response missing permission $permission" >&2
      printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
      exit 1
    fi
  done
}

auth_get() {
  local token="$1"
  local path="$2"
  local response status payload

  if ! response="$(curl -sS -w $'\n%{http_code}' -X GET "${BASE_URL}${path}" \
    -H "Authorization: Bearer ${token}")"; then
    fail "curl failed for GET $path"
  fi

  status="$(printf '%s' "$response" | tail -n 1)"
  payload="$(printf '%s' "$response" | sed '$d')"

  if [[ "$status" != "200" ]]; then
    echo "FAILED: GET $path returned HTTP $status" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi

  if ! printf '%s\n' "$payload" | jq -e '.success == true' >/dev/null; then
    echo "FAILED: GET $path response should have success=true" >&2
    printf '%s\n' "$payload" | jq . 2>/dev/null || printf '%s\n' "$payload"
    exit 1
  fi
}

need psql
load_dotenv_database_url
[[ -n "$DB_URL" ]] || fail "set IMS_DATABASE_URL or DATABASE_URL"

expect_no_rows "demo users exist" "
WITH expected(username) AS (
  VALUES
    ('admin'),
    ('wms01'),
    ('warehouse01'),
    ('purchaser01'),
    ('sales01'),
    ('auditor01'),
    ('qm01'),
    ('planner01')
)
SELECT e.username
FROM expected e
LEFT JOIN sys.sys_users u ON u.username = e.username
WHERE u.user_id IS NULL;
"

expect_no_rows "demo users have non-placeholder Argon2 hashes" "
WITH expected(username) AS (
  VALUES
    ('admin'),
    ('wms01'),
    ('warehouse01'),
    ('purchaser01'),
    ('sales01'),
    ('auditor01'),
    ('qm01'),
    ('planner01')
)
SELECT e.username
FROM expected e
JOIN sys.sys_users u ON u.username = e.username
WHERE u.password_hash = 'demo-not-for-production'
   OR u.password_hash NOT LIKE (chr(36) || 'argon2%');
"

expect_no_rows "audit role permissions are present exactly once" "
WITH expected(role_id, permission_code) AS (
  VALUES
    ('ADMIN', 'audit:read'),
    ('AUDITOR', 'audit:read')
),
counts AS (
  SELECT
    p.role_id,
    p.permission_code,
    COUNT(*) AS row_count,
    COUNT(*) FILTER (
      WHERE COALESCE(p.granted, true)
        AND (p.expires_at IS NULL OR p.expires_at > now())
    ) AS active_count
  FROM sys.sys_user_permissions p
  JOIN expected e
    ON e.role_id = p.role_id
   AND e.permission_code = p.permission_code
  WHERE p.user_id IS NULL
  GROUP BY p.role_id, p.permission_code
)
SELECT
  e.role_id || ':' || e.permission_code
  || ' row_count=' || COALESCE(c.row_count, 0)::text
  || ' active_count=' || COALESCE(c.active_count, 0)::text
FROM expected e
LEFT JOIN counts c
  ON c.role_id = e.role_id
 AND c.permission_code = e.permission_code
WHERE COALESCE(c.row_count, 0) <> 1
   OR COALESCE(c.active_count, 0) <> 1;
"

expect_no_rows "system parameter role permissions are present exactly once" "
WITH expected(role_id, permission_code) AS (
  VALUES
    ('ADMIN', 'system-param:read'),
    ('ADMIN', 'system-param:write')
),
counts AS (
  SELECT
    p.role_id,
    p.permission_code,
    COUNT(*) AS row_count,
    COUNT(*) FILTER (
      WHERE COALESCE(p.granted, true)
        AND (p.expires_at IS NULL OR p.expires_at > now())
    ) AS active_count
  FROM sys.sys_user_permissions p
  JOIN expected e
    ON e.role_id = p.role_id
   AND e.permission_code = p.permission_code
  WHERE p.user_id IS NULL
  GROUP BY p.role_id, p.permission_code
)
SELECT
  e.role_id || ':' || e.permission_code
  || ' row_count=' || COALESCE(c.row_count, 0)::text
  || ' active_count=' || COALESCE(c.active_count, 0)::text
FROM expected e
LEFT JOIN counts c
  ON c.role_id = e.role_id
 AND c.permission_code = e.permission_code
WHERE COALESCE(c.row_count, 0) <> 1
   OR COALESCE(c.active_count, 0) <> 1;
"

expect_no_rows "quality role permissions from demo-login patch are present exactly once" "
WITH expected(role_id, permission_code) AS (
  VALUES
    ('ADMIN', 'quality:read'),
    ('ADMIN', 'quality:write'),
    ('ADMIN', 'quality:decision'),
    ('QM_USER', 'quality:read'),
    ('QM_USER', 'quality:write'),
    ('QM_USER', 'quality:decision')
),
counts AS (
  SELECT
    p.role_id,
    p.permission_code,
    COUNT(*) AS row_count,
    COUNT(*) FILTER (
      WHERE COALESCE(p.granted, true)
        AND (p.expires_at IS NULL OR p.expires_at > now())
    ) AS active_count
  FROM sys.sys_user_permissions p
  JOIN expected e
    ON e.role_id = p.role_id
   AND e.permission_code = p.permission_code
  WHERE p.user_id IS NULL
  GROUP BY p.role_id, p.permission_code
)
SELECT
  e.role_id || ':' || e.permission_code
  || ' row_count=' || COALESCE(c.row_count, 0)::text
  || ' active_count=' || COALESCE(c.active_count, 0)::text
FROM expected e
LEFT JOIN counts c
  ON c.role_id = e.role_id
 AND c.permission_code = e.permission_code
WHERE COALESCE(c.row_count, 0) <> 1
   OR COALESCE(c.active_count, 0) <> 1;
"

has_sqlx_table="$(
  psql_scalar "
SELECT EXISTS (
  SELECT 1
  FROM information_schema.tables
  WHERE table_schema = 'public'
    AND table_name = '_sqlx_migrations'
)::text;
"
)"

if [[ "$has_sqlx_table" == "true" ]]; then
  expect_no_rows "patch migrations are recorded by SQLx" "
WITH expected(version) AS (
  VALUES (3), (7), (8)
),
state AS (
  SELECT version, success
  FROM public._sqlx_migrations
  WHERE version IN (3, 7, 8)
)
SELECT
  e.version::text
  || ' success=' || COALESCE(s.success::text, 'missing')
FROM expected e
LEFT JOIN state s ON s.version = e.version
WHERE s.version IS NULL
   OR s.success IS DISTINCT FROM true;
"
else
  echo "SKIP: public._sqlx_migrations is absent; data checks passed, but SQLx bookkeeping is not verified."
fi

if [[ "$VERIFY_PATCH_API" == "1" ]]; then
  need curl
  need jq

  admin_payload="$(login admin)"
  auditor_payload="$(login auditor01)"
  qm_payload="$(login qm01)"

  assert_permissions "$admin_payload" audit:read system-param:read system-param:write quality:read quality:write quality:decision
  assert_permissions "$auditor_payload" audit:read
  assert_permissions "$qm_payload" quality:read quality:write quality:decision

  admin_token="$(printf '%s\n' "$admin_payload" | jq -r '.data.access_token')"
  auditor_token="$(printf '%s\n' "$auditor_payload" | jq -r '.data.access_token')"

  auth_get "$admin_token" "/api/system/params?page=1&page_size=1"
  auth_get "$auditor_token" "/api/system/audit-logs?page=1&page_size=1"

  echo "OK: API demo login and permission-gated routes"
else
  echo "SKIP: API login checks disabled. Set VERIFY_PATCH_API=1 to verify demo password through /api/auth/login."
fi
