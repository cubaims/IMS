#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
TOKEN="${TOKEN:-}"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

echo "Refresh reports"

REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/reports/refresh" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$REFRESH_RESPONSE" | jq

SUCCESS=$(echo "$REFRESH_RESPONSE" | jq -r '.success')

if [ "$SUCCESS" != "true" ]; then
  echo "Report refresh failed"
  exit 1
fi

echo "Query current stock"

CURRENT_STOCK_RESPONSE=$(curl -s "$BASE_URL/api/reports/current-stock" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$CURRENT_STOCK_RESPONSE" | jq

echo "Query stock by zone"

STOCK_BY_ZONE_RESPONSE=$(curl -s "$BASE_URL/api/reports/stock-by-zone" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$STOCK_BY_ZONE_RESPONSE" | jq

echo "Query bin stock summary"

BIN_SUMMARY_RESPONSE=$(curl -s "$BASE_URL/api/reports/bin-stock-summary" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$BIN_SUMMARY_RESPONSE" | jq

echo "Query batch stock summary"

BATCH_SUMMARY_RESPONSE=$(curl -s "$BASE_URL/api/reports/batch-stock-summary" \
  ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"})

echo "$BATCH_SUMMARY_RESPONSE" | jq

echo "Report refresh test passed"
