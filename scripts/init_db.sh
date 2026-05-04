#!/usr/bin/env bash
set -euo pipefail

# 获取脚本所在目录的绝对路径
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DATABASE_URL="${DATABASE_URL:-postgres://pgadmin:StrongPass%402026@10.0.0.10:5432/ims_workspace}"
psql "$DATABASE_URL" -f "$PROJECT_ROOT/migrations/0001_schema_final_ultimate_complete_v9.sql"
