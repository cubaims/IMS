# Phase 6 Production Order Acceptance Tests

## Status: ⚠️ NEEDS IMPLEMENTATION FIXES

Phase 6 production order functionality exists in the codebase but requires fixes similar to Phase 5:

### Issues to Fix

1. **Field Name Mismatch** - `state.pool` should be `state.db_pool`
   - File: `crates/cuba-production/src/interface/handlers.rs`
   - Line 26: `let repo = Arc::new(PostgresProductionRepository::new(state.pool.clone()));`
   - Fix: Change to `state.db_pool.clone()`

2. **Error Mapping** - Need to add `map_inventory_db_error` for inventory transactions
   - File: `crates/cuba-production/src/infrastructure/postgres.rs`
   - Apply same pattern as Phase 5 (purchase/sales modules)

3. **Lock Timeouts** - Add transaction lock timeouts
   - `complete_order()` should have `SET LOCAL lock_timeout = '10s'`
   - `release_order()` should have `SET LOCAL lock_timeout = '5s'`

4. **Chrono Serde Feature** - May need to enable in Cargo.toml
   - Check `crates/cuba-production/Cargo.toml`
   - Add `chrono = { version = "0.4", features = ["serde"] }` if needed

### Test Coverage

Phase 6 covers production order flows:

- BOM explosion preview
- Production order creation
- Production order release
- Production order completion (one-click)
- Component consumption (261 movement)
- Finished goods receipt (101 movement)
- Batch genealogy tracking
- Cost variance calculation
- Report refresh verification

## Prerequisites

- API server running
- PostgreSQL initialized with IMS v9 schema
- Master data available:
  - Variant: `FIN-A001`
  - Finished material: `FIN001`
  - BOM: `BOM-FIN-A01`
  - Work center: `WC-ASSY-01`
  - Finished goods bin: `FG-A01`
  - Component materials with inventory
- `jq` installed locally

## Run

```bash
BASE_URL=http://localhost:8080 ./tests/phase6/phase6_acceptance.sh
```

If auth is enabled:

```bash
BASE_URL=http://localhost:8080 TOKEN=<access_token> ./tests/phase6/phase6_acceptance.sh
```

## Expected Results

### BOM Explosion
- Returns list of components with required_qty, available_qty, shortage_qty
- Merges components if requested

### Production Order Creation
- Returns order_id with status "PLANNED"
- Creates component lines based on BOM

### Production Order Release
- Changes status from "PLANNED" to "RELEASED"
- Validates component availability

### Production Order Completion
- Changes status to "COMPLETED"
- Creates 101 transaction for finished goods
- Creates 261 transactions for component consumption
- Records batch genealogy
- Calculates cost variance
- Sets reports_stale = true

### Batch Genealogy
- Links finished batch to component batches
- Enables forward/backward traceability

### Cost Variance
- Compares planned vs actual material costs
- Tracks labor and overhead variances

## Verification Commands

Individual verification commands from the acceptance script:

```bash
# 1. Health check
curl http://localhost:8080/health

# 2. Version check
curl http://localhost:8080/api/version

# 3. BOM explosion
curl -X POST http://localhost:8080/api/production/bom-explosion \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "quantity": 10,
    "merge_components": true
  }'

# 4. Create production order
curl -X POST http://localhost:8080/api/production-orders \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "bom_id": "BOM-FIN-A01",
    "planned_qty": 10,
    "work_center_id": "WC-ASSY-01",
    "planned_start_date": "2026-05-05",
    "planned_end_date": "2026-05-08",
    "remark": "phase 6 production order"
  }'

# Save ORDER_ID from response
export ORDER_ID="MO-xxxxxxxx"

# 5. Query order details
curl http://localhost:8080/api/production-orders/$ORDER_ID

# 6. Query order components
curl http://localhost:8080/api/production-orders/$ORDER_ID/components

# 7. Release order
curl -X POST http://localhost:8080/api/production-orders/$ORDER_ID/release \
  -H "Content-Type: application/json" \
  -d '{"remark": "release phase 6 production order"}'

# 8. Complete order
curl -X POST http://localhost:8080/api/production-orders/$ORDER_ID/complete \
  -H "Content-Type: application/json" \
  -d '{
    "completed_qty": 10,
    "finished_batch_number": "BATCH-FIN001-P6-001",
    "finished_to_bin": "FG-A01",
    "posting_date": "2026-05-04T14:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "phase 6 production complete"
  }'

# 9. Query genealogy
curl http://localhost:8080/api/production-orders/$ORDER_ID/genealogy

# 10. Query batch components
curl http://localhost:8080/api/production/batches/BATCH-FIN001-P6-001/components

# 11. Query variance
curl http://localhost:8080/api/production-orders/$ORDER_ID/variance

# 12. Refresh reports
curl -X POST http://localhost:8080/api/reports/refresh

# 13. Query stock
curl "http://localhost:8080/api/reports/current-stock?material_id=FIN001"
```

## Next Steps

To make Phase 6 functional:

1. Fix `state.pool` → `state.db_pool` in handlers.rs
2. Add error mapping in postgres.rs
3. Add lock timeouts to transactions
4. Test compilation: `cargo build -p cuba-api`
5. Run acceptance tests
6. Commit changes

## Related Documentation

- Phase 5 completion: `PHASE5_COMPLETION.md`
- Phase 5 tests: `tests/phase5/`
- Error mapping: `crates/cuba-shared/src/db_error.rs`
