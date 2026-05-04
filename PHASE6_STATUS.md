# Phase 6 Status Report

## Current Status: ⚠️ SKELETON EXISTS - NEEDS FIXES

Phase 6 (Production Order Management) has been **partially implemented** but requires the same fixes that were applied to Phase 5.

### What Exists ✅

1. **Crate Structure** - `crates/cuba-production/` exists with full module structure
2. **Handlers** - All 13 handlers implemented in `interface/handlers.rs`:
   - `preview_bom_explosion` - BOM explosion preview
   - `create_production_order` - Create production order
   - `get_production_order` - Query order details
   - `list_production_orders` - List orders with filters
   - `release_production_order` - Release order for execution
   - `cancel_production_order` - Cancel order
   - `close_production_order` - Close order
   - `complete_production_order` - One-click completion
   - `get_order_components` - Query component lines
   - `get_order_genealogy` - Query batch genealogy
   - `get_components_by_finished_batch` - Forward traceability
   - `get_where_used_by_component_batch` - Backward traceability
   - `get_order_variance` - Query cost variance
   - `list_production_variances` - List variances

3. **Routes Registered** - Production routes registered in `main.rs`:
   - `/api/production/*` - Production utilities
   - `/api/production-orders/*` - Production order operations

4. **Test Script** - Created `tests/phase6/phase6_acceptance.sh` with 13 verification steps

### What Needs Fixing ⚠️

#### 1. Field Name Mismatch (Critical)

**File:** `crates/cuba-production/src/interface/handlers.rs`

**Issue:** Line 26 uses `state.pool` but should be `state.db_pool`

```rust
// Current (WRONG):
let repo = Arc::new(PostgresProductionRepository::new(state.pool.clone()));

// Should be:
let repo = Arc::new(PostgresProductionRepository::new(state.db_pool.clone()));
```

**Impact:** Service will not compile or will crash at runtime

#### 2. Error Mapping (Important)

**File:** `crates/cuba-production/src/infrastructure/postgres.rs`

**Issue:** Inventory transactions need error mapping

**Fix:** Add `map_inventory_db_error` to all `post_inventory_transaction` calls:

```rust
// Add import:
use cuba_shared::{map_inventory_db_error, AppError, AppResult};

// Change:
.execute(&mut *tx)
.await?;

// To:
.execute(&mut *tx)
.await
.map_err(map_inventory_db_error)?;
```

**Impact:** Database errors won't be properly mapped to business error codes

#### 3. Lock Timeouts (Important)

**File:** `crates/cuba-production/src/infrastructure/postgres.rs`

**Issue:** Missing transaction lock timeouts

**Fix:** Add lock timeouts to transactions:

```rust
// In complete_order():
let mut tx = self.pool.begin().await?;
sqlx::query("SET LOCAL lock_timeout = '10s'")
    .execute(&mut *tx)
    .await?;

// In release_order():
let mut tx = self.pool.begin().await?;
sqlx::query("SET LOCAL lock_timeout = '5s'")
    .execute(&mut *tx)
    .await?;
```

**Impact:** Transactions may hang indefinitely under high concurrency

#### 4. Chrono Serde Feature (Minor)

**File:** `crates/cuba-production/Cargo.toml`

**Issue:** May need chrono serde feature for date serialization

**Fix:** Check and add if needed:

```toml
chrono = { version = "0.4", features = ["serde"] }
```

**Impact:** Date fields may not serialize/deserialize correctly

#### 5. Status Error Codes (Enhancement)

**File:** `crates/cuba-production/src/infrastructure/postgres.rs`

**Issue:** Status validation should use business error codes

**Fix:** Similar to Phase 5:

```rust
fn validate_release_status(status: &str) -> AppResult<()> {
    match status {
        "PLANNED" => Ok(()),
        "RELEASED" => Err(AppError::business(
            "MO_STATUS_INVALID",
            "生产订单已下达，不能重复下达",
        )),
        "COMPLETED" => Err(AppError::business(
            "MO_STATUS_INVALID",
            "生产订单已完成，不能下达",
        )),
        // ... other statuses
    }
}
```

**Impact:** Error messages less clear, harder to handle in frontend

### Acceptance Test Coverage

The `tests/phase6/phase6_acceptance.sh` script tests:

1. ✅ Health check
2. ✅ Version check
3. ✅ BOM explosion preview
4. ✅ Production order creation
5. ✅ Production order query
6. ✅ Component query
7. ✅ Order release
8. ✅ Order completion (one-click)
9. ✅ Batch genealogy query
10. ✅ Finished batch component query
11. ✅ Cost variance query
12. ✅ Report refresh
13. ✅ Stock verification

### Expected Behavior

#### BOM Explosion
- Input: variant_code, finished_material_id, quantity
- Output: List of components with required_qty, available_qty, shortage_qty
- Validates: Component availability before production

#### Production Order Creation
- Input: variant, material, BOM, quantity, work center, dates
- Output: order_id with status "PLANNED"
- Creates: Order header + component lines from BOM

#### Production Order Release
- Input: order_id
- Output: status changed to "RELEASED"
- Validates: Component availability, order status

#### Production Order Completion
- Input: order_id, completed_qty, batch_number, bin, posting_date
- Output: status "COMPLETED", transaction IDs, genealogy count, variance ID
- Creates:
  - 101 transaction (finished goods receipt)
  - 261 transactions (component consumption via FEFO)
  - Batch genealogy records
  - Cost variance record
- Sets: reports_stale = true

#### Batch Genealogy
- Links finished batch to component batches
- Enables forward traceability (finished → components)
- Enables backward traceability (component → finished)

#### Cost Variance
- Compares planned vs actual material costs
- Tracks labor and overhead variances
- Identifies over/under budget production

### Database Functions

Phase 6 relies on PostgreSQL functions (assumed to exist in v9 schema):

- `wms.fn_preview_bom_explosion()` - BOM explosion calculation
- `wms.fn_pick_batch_fefo()` - FEFO component picking
- `wms.post_inventory_transaction()` - Inventory posting
- `wms.fn_calculate_production_variance()` - Variance calculation

### Quick Fix Checklist

To make Phase 6 functional:

- [ ] Fix `state.pool` → `state.db_pool` in handlers.rs
- [ ] Add `map_inventory_db_error` import in postgres.rs
- [ ] Add error mapping to inventory transaction calls
- [ ] Add lock timeouts to complete_order() and release_order()
- [ ] Check chrono serde feature in Cargo.toml
- [ ] Add business error codes for status validation
- [ ] Test compilation: `cargo build -p cuba-api`
- [ ] Run acceptance test: `./tests/phase6/phase6_acceptance.sh`
- [ ] Verify all 13 test steps pass
- [ ] Commit changes

### Estimated Effort

- **Time:** 30-60 minutes
- **Complexity:** Low (same patterns as Phase 5)
- **Risk:** Low (well-tested patterns)

### Files to Modify

1. `crates/cuba-production/src/interface/handlers.rs` - Fix state.pool
2. `crates/cuba-production/src/infrastructure/postgres.rs` - Add error mapping + timeouts
3. `crates/cuba-production/Cargo.toml` - Check chrono feature
4. `tests/phase6/phase6_acceptance.sh` - Already created ✅
5. `tests/phase6/README.md` - Already created ✅

### Comparison with Phase 5

Phase 6 follows the same patterns as Phase 5:

| Aspect | Phase 5 | Phase 6 |
|--------|---------|---------|
| Field name | `state.db_pool` ✅ | `state.pool` ❌ |
| Error mapping | `map_inventory_db_error` ✅ | Missing ❌ |
| Lock timeouts | 2s/5s ✅ | Missing ❌ |
| Chrono serde | Enabled ✅ | Unknown ❓ |
| Business errors | Implemented ✅ | Partial ❓ |
| Test scripts | Complete ✅ | Created ✅ |

### Next Steps

1. **Apply Phase 5 fixes to Phase 6** - Use same patterns
2. **Test compilation** - Ensure no errors
3. **Run acceptance tests** - Verify all endpoints work
4. **Document results** - Update this file with test results
5. **Commit Phase 6** - Create git commit similar to Phase 5

### Related Files

- Phase 5 completion: `PHASE5_COMPLETION.md`
- Phase 5 checklist: `PHASE5_CHECKLIST.md`
- Phase 5 tests: `tests/phase5/`
- Error mapper: `crates/cuba-shared/src/db_error.rs`
- Phase 6 tests: `tests/phase6/` ✅

---

**Status:** Ready for fixes (30-60 min effort)
**Priority:** Medium (Phase 5 is complete and working)
**Blocker:** None (Phase 5 can be used independently)
