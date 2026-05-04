#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"

echo "== IMS Workspace Phase 3 Master Data Smoke Test =="

echo "1. health check"
curl -s "$BASE_URL/health"
echo ""

echo "2. version check (skipped - endpoint not implemented yet)"
# curl -s "$BASE_URL/api/version" | jq .

echo "3. create material RM-TEST-001"
curl -s -X POST "$BASE_URL/api/master-data/materials" \
  -H "Content-Type: application/json" \
  -d '{
    "material_id": "RMTEST001",
    "material_name": "Test Raw Material",
    "material_type": "原材料",
    "base_unit": "PCS",
    "default_zone": "RM",
    "safety_stock": 100,
    "reorder_point": 50,
    "standard_price": "10.00",
    "map_price": "10.00"
  }' | jq .

echo "4. get material RMTEST001"
curl -s "$BASE_URL/api/master-data/materials/RMTEST001" | jq .

echo "5. create finished material FINTEST001"
curl -s -X POST "$BASE_URL/api/master-data/materials" \
  -H "Content-Type: application/json" \
  -d '{
    "material_id": "FINTEST001",
    "material_name": "Test Finished Product",
    "material_type": "成品",
    "base_unit": "PCS",
    "default_zone": "FG",
    "safety_stock": 20,
    "reorder_point": 10,
    "standard_price": "100.00",
    "map_price": "100.00"
  }' | jq .

echo "6. create bin RM-T01"
curl -s -X POST "$BASE_URL/api/master-data/bins" \
  -H "Content-Type: application/json" \
  -d '{
    "bin_code": "RM-T01",
    "zone": "RM",
    "bin_type": "普通货位",
    "capacity": 10000,
    "notes": "Smoke test raw material bin"
  }' | jq .

echo "7. create bin FG-T01"
curl -s -X POST "$BASE_URL/api/master-data/bins" \
  -H "Content-Type: application/json" \
  -d '{
    "bin_code": "FG-T01",
    "zone": "FG",
    "bin_type": "普通货位",
    "capacity": 10000,
    "notes": "Smoke test finished goods bin"
  }' | jq .

echo "8. create supplier SUPTEST001"
curl -s -X POST "$BASE_URL/api/master-data/suppliers" \
  -H "Content-Type: application/json" \
  -d '{
    "supplier_id": "SUPTEST001",
    "supplier_name": "Smoke Test Supplier",
    "contact_person": "Tester",
    "phone": "13800000000",
    "email": "supplier@example.com",
    "address": "Test Address",
    "quality_rating": "A"
  }' | jq .

echo "9. bind material supplier"
curl -s -X POST "$BASE_URL/api/master-data/materials/RMTEST001/suppliers" \
  -H "Content-Type: application/json" \
  -d '{
    "material_id": "RMTEST001",
    "supplier_id": "SUPTEST001",
    "is_primary": true,
    "supplier_material_code": "SUP-RM-001",
    "purchase_price": "9.50",
    "currency": "CNY",
    "lead_time_days": 7,
    "moq": 100,
    "quality_rating": "A"
  }' | jq .

echo "10. create customer CUSTTEST001"
curl -s -X POST "$BASE_URL/api/master-data/customers" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "CUSTTEST001",
    "customer_name": "Smoke Test Customer",
    "contact_person": "Buyer",
    "phone": "13900000000",
    "email": "customer@example.com",
    "address": "Customer Address",
    "credit_limit": "100000.00"
  }' | jq .

echo "11. create BOM BOMTEST001"
curl -s -X POST "$BASE_URL/api/master-data/boms" \
  -H "Content-Type: application/json" \
  -d '{
    "bom_id": "BOMTEST001",
    "bom_name": "Smoke Test BOM",
    "parent_material_id": "FINTEST001",
    "variant_code": null,
    "version": "1.0",
    "base_quantity": "1.00",
    "valid_from": "2026-05-01",
    "valid_to": null,
    "status": "草稿",
    "notes": "Smoke test BOM"
  }' | jq .

echo "12. add BOM component"
curl -s -X POST "$BASE_URL/api/master-data/boms/BOMTEST001/components" \
  -H "Content-Type: application/json" \
  -d '{
    "bom_id": "BOMTEST001",
    "parent_material_id": "FINTEST001",
    "component_material_id": "RMTEST001",
    "quantity": "2.00",
    "unit": "PCS",
    "bom_level": 1,
    "scrap_rate": "0.00",
    "is_critical": true
  }' | jq .

echo "13. validate BOM"
curl -s -X POST "$BASE_URL/api/master-data/boms/BOMTEST001/validate" | jq .

echo "14. get BOM tree"
curl -s "$BASE_URL/api/master-data/boms/BOMTEST001/tree" | jq .

echo "15. create product variant"
curl -s -X POST "$BASE_URL/api/master-data/product-variants" \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FINTEST-A",
    "variant_name": "Smoke Test Variant A",
    "base_material_id": "FINTEST001",
    "bom_id": "BOMTEST001",
    "standard_cost": "120.00"
  }' | jq .

echo "16. create work center"
curl -s -X POST "$BASE_URL/api/master-data/work-centers" \
  -H "Content-Type: application/json" \
  -d '{
    "work_center_id": "WCTEST001",
    "work_center_name": "Smoke Test Work Center",
    "location": "Workshop A",
    "capacity_per_day": 1000,
    "efficiency": "100.00"
  }' | jq .

echo "17. create inspection characteristic"
curl -s -X POST "$BASE_URL/api/master-data/inspection-chars" \
  -H "Content-Type: application/json" \
  -d '{
    "char_id": "CHARTEST001",
    "char_name": "Smoke Test Dimension",
    "material_type": "原材料",
    "inspection_type": "尺寸",
    "method": "Caliper",
    "standard": "10±0.2",
    "unit": "mm",
    "lower_limit": "9.80",
    "upper_limit": "10.20",
    "is_critical": true
  }' | jq .

echo "18. create defect code"
curl -s -X POST "$BASE_URL/api/master-data/defect-codes" \
  -H "Content-Type: application/json" \
  -d '{
    "defect_code": "DEFTEST001",
    "defect_name": "Smoke Test Scratch",
    "category": "外观",
    "severity": "一般",
    "description": "Smoke test defect code"
  }' | jq .

echo "19. list all major master data"
curl -s "$BASE_URL/api/master-data/materials" | jq .
curl -s "$BASE_URL/api/master-data/bins" | jq .
curl -s "$BASE_URL/api/master-data/suppliers" | jq .
curl -s "$BASE_URL/api/master-data/customers" | jq .
curl -s "$BASE_URL/api/master-data/boms" | jq .
curl -s "$BASE_URL/api/master-data/product-variants" | jq .
curl -s "$BASE_URL/api/master-data/work-centers" | jq .
curl -s "$BASE_URL/api/master-data/inspection-chars" | jq .
curl -s "$BASE_URL/api/master-data/defect-codes" | jq .

echo "== Phase 3 smoke test completed =="