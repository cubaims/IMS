# Phase 5 Acceptance Tests

Phase 5 covers inbound and outbound flows:

- PO creation
- PO receipt posting
- SO creation
- SO shipment posting
- FEFO batch picking
- Insufficient stock blocking
- Inventory report refresh verification

## Required

- API server running
- PostgreSQL initialized with IMS v9 schema
- Master data available:
  - supplier `SUP-001`
  - customer `CUST-001`
  - material `CG001`
  - bin `RM-A01`
- `jq` installed locally

## Run

```bash
BASE_URL=http://localhost:8080 ./tests/phase5/phase5_acceptance.sh
```

If auth is enabled:

```bash
BASE_URL=http://localhost:8080 TOKEN=<access_token> ./tests/phase5/phase5_acceptance.sh
```

## Expected

- PO receipt returns 101
- SO shipment returns 261
- FEFO returns available batch
- Insufficient stock returns business error
- Reports refresh succeeds
- Current stock changes correctly

## Individual Tests

```bash
# Test PO receipt
./tests/phase5/po_receipt_test.sh

# Test FEFO picking
./tests/phase5/fefo_pick_test.sh

# Test SO shipment
./tests/phase5/so_shipment_test.sh

# Test insufficient stock blocking
./tests/phase5/insufficient_stock_test.sh

# Test report refresh
./tests/phase5/report_refresh_test.sh
```

## SQL Verification

```bash
psql "$DATABASE_URL" -f tests/phase5/verify_phase5.sql
```
