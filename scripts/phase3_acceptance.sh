#!/usr/bin/env bash
set -euo pipefail

exec "$(dirname "$0")/smoke_master_data.sh" "$@"
