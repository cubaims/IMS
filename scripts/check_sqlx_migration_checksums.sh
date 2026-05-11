#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MIGRATIONS_DIR="${IMS_MIGRATIONS_DIR:-$PROJECT_ROOT/migrations}"
DB_URL="${IMS_DATABASE_URL:-${DATABASE_URL:-}}"
EMIT_REPAIR_SQL=0
ALLOW_PENDING="${ALLOW_PENDING:-0}"

usage() {
  cat <<'EOF'
Usage: ./scripts/check_sqlx_migration_checksums.sh [--repair-sql]

Compares local SQLx migration files with public._sqlx_migrations using the
SQLx 0.8 checksum algorithm: SHA-384 over the SQL file bytes.

Environment:
  IMS_DATABASE_URL or DATABASE_URL   PostgreSQL connection string.
  IMS_MIGRATIONS_DIR                 Override migrations directory.
  ALLOW_PENDING=1                    Do not fail for pending local migrations.

Options:
  --repair-sql                       Print guarded UPDATE statements for
                                     checksum mismatches. It does not execute
                                     any repair.
EOF
}

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

checksum_hex() {
  local file="$1"
  openssl dgst -sha384 -binary "$file" | xxd -p -c 256
}

record_issue() {
  issues=$((issues + 1))
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repair-sql)
      EMIT_REPAIR_SQL=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown argument: $1"
      ;;
  esac
done

need psql
need openssl
need xxd
load_dotenv_database_url

[[ -n "$DB_URL" ]] || fail "set IMS_DATABASE_URL or DATABASE_URL"
[[ -d "$MIGRATIONS_DIR" ]] || fail "migrations directory not found: $MIGRATIONS_DIR"

state="$(
  psql_scalar "
SELECT
  EXISTS (
    SELECT 1
    FROM information_schema.schemata
    WHERE schema_name IN ('mdm', 'wms', 'rpt', 'sys')
  )::text,
  EXISTS (
    SELECT 1
    FROM information_schema.tables
    WHERE table_schema = 'public'
      AND table_name = '_sqlx_migrations'
  )::text;
"
)"

IFS='|' read -r has_ims_schema has_sqlx_table <<< "$state"

if [[ "$has_sqlx_table" != "true" ]]; then
  if [[ "$has_ims_schema" == "true" ]]; then
    echo "MISSING_SQLX_TABLE: IMS schemas exist but public._sqlx_migrations is absent."
    echo "Do not run sqlx migrate until the baseline is repaired; see docs/database-migrations.md."
    exit 2
  fi

  echo "OK: no IMS schemas and no SQLx migration table; database has no SQLx checksum state yet."
  exit 0
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
LOCAL_STATE="$TMP_DIR/local.tsv"
LOCAL_SORTED="$TMP_DIR/local.sorted.tsv"
DB_STATE="$TMP_DIR/db.tsv"
DB_SORTED="$TMP_DIR/db.sorted.tsv"

while IFS= read -r file; do
  base="$(basename "$file")"
  prefix="${base%%_*}"

  if [[ ! "$prefix" =~ ^[0-9]+$ ]]; then
    continue
  fi

  version="$((10#$prefix))"
  desc_part="${base#*_}"
  desc="${desc_part%.sql}"
  desc="${desc//_/ }"

  printf '%s\t%s\t%s\t%s\n' "$version" "$base" "$desc" "$(checksum_hex "$file")" >> "$LOCAL_STATE"
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '[0-9]*_*.sql' -print | sort)

touch "$LOCAL_STATE"

duplicate_versions="$(cut -f1 "$LOCAL_STATE" | sort -n | uniq -d)"
if [[ -n "$duplicate_versions" ]]; then
  echo "Duplicate local migration versions:" >&2
  printf '%s\n' "$duplicate_versions" >&2
  exit 1
fi

sort -n -k1,1 "$LOCAL_STATE" > "$LOCAL_SORTED"

psql "$DB_URL" -X -v ON_ERROR_STOP=1 -Atq -F $'\t' -c "
SELECT version::text, description, success::text, encode(checksum, 'hex')
FROM public._sqlx_migrations
ORDER BY version;
" > "$DB_STATE"

sort -n -k1,1 "$DB_STATE" > "$DB_SORTED"

echo "SQLx migration checksum state:"

awk -F '\t' \
  -v allow_pending="$ALLOW_PENDING" \
  -v emit_repair_sql="$EMIT_REPAIR_SQL" '
  FILENAME == ARGV[1] {
    db_desc[$1] = $2
    db_success[$1] = $3
    db_checksum[$1] = $4
    db_seen[$1] = 1
    db_versions[++db_count] = $1
    next
  }

  FILENAME == ARGV[2] {
    version = $1
    file = $2
    desc = $3
    checksum = $4
    local_seen[version] = 1

    if (!(version in db_seen)) {
      print "PENDING: version=" version " file=" file
      pending_count++
      if (allow_pending != "1") {
        issues++
      }
      next
    }

    if (db_success[version] != "true") {
      print "FAILED: version=" version " file=" file " success=" db_success[version]
      issues++
      next
    }

    if (db_checksum[version] != checksum) {
      print "CHECKSUM_MISMATCH: version=" version " file=" file
      print "  db=" db_checksum[version]
      print "  fs=" checksum
      repair_versions[++repair_count] = version
      repair_checksum[version] = checksum
      issues++
      next
    }

    if (db_desc[version] != desc) {
      print "DESCRIPTION_DRIFT: version=" version " file=" file
      print "  db=" db_desc[version]
      print "  fs=" desc
      issues++
      next
    }

    print "OK: version=" version " file=" file
    ok_count++
    next
  }

  END {
    for (idx = 1; idx <= db_count; idx++) {
      version = db_versions[idx]
      if (!(version in local_seen)) {
        print "APPLIED_ONLY: version=" version " description=" db_desc[version]
        issues++
      }
    }

    print "Summary: ok=" ok_count + 0 " pending=" pending_count + 0 " issues=" issues + 0

    if (emit_repair_sql == "1") {
      if (repair_count == 0) {
        print "-- No checksum mismatch repair SQL to emit."
      } else {
        print "-- Review before execution. Only run after confirming the database effects"
        print "-- already match the frozen migration files. This updates SQLx bookkeeping only."
        print "BEGIN;"
        for (idx = 1; idx <= repair_count; idx++) {
          version = repair_versions[idx]
          print "UPDATE public._sqlx_migrations"
          print "SET checksum = decode('\''" repair_checksum[version] "'\'', '\''hex'\'')"
          print "WHERE version = " version
          print "  AND encode(checksum, '\''hex'\'') = '\''" db_checksum[version] "'\''"
          print "  AND success = true;"
        }
        print "COMMIT;"
      }
    }

    if (issues > 0) {
      exit 1
    }
  }
' "$DB_SORTED" "$LOCAL_SORTED"
