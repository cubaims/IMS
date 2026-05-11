# IMS Workspace Server

Rust 1.95 / Edition 2024 service skeleton for IMS (Workspace), using DDD + Clean Architecture.

## Architecture

This project uses a module-first workspace layout. Each bounded-context crate owns its own clean architecture layers:

```text
crates/cuba-{module}/
├── src/domain/          # Entities, value objects, domain errors/rules
├── src/application/     # Use cases, ports, commands, service orchestration
├── src/infrastructure/  # PostgreSQL adapters and external dependencies
├── src/interface/       # DTOs, HTTP routes, handlers
└── src/lib.rs
```

The database schema and incremental SQL migrations are under `migrations/`.

## Modules

### Core Business Modules

- **cuba-auth** - Authentication & Authorization (JWT, RBAC)
- **cuba-master-data** - Master Data Management (Materials, BOMs, Customers, Suppliers, etc.)
- **cuba-inventory** - Inventory Management (Stock, Movements, Bins, Batches)
- **cuba-purchase** - Purchase Order Management (PO, Receipts, GR/IR)
- **cuba-sales** - Sales Order Management (SO, Shipments, Invoicing)
- **cuba-production** - Production Order Management (MO, BOM Explosion, One-click Completion)
- **cuba-quality** - Quality Management (Inspections, QC Results)
- **cuba-mrp** - Material Requirements Planning (MRP Run, Shortage Analysis)
- **cuba-reporting** - Reporting & Analytics (Materialized Views, Data Refresh)

### Infrastructure Modules

- **cuba-api** - Main API Gateway (Axum HTTP Server)
- **cuba-worker** - Background Job Worker (Async Tasks)
- **cuba-shared** - Shared Types & Utilities (AppState, Errors, DB Helpers)

## Run locally

Set the minimum environment first, either in `.env` or in your shell:

```bash
DATABASE_URL=postgres://ims:ims@localhost:5432/ims_workspace
```

For an empty or SQLx-managed database, run migrations as an explicit command,
then start the API:

```bash
cargo run -p cuba-api -- migrate
cargo run -p cuba-api
```

If the database was initialized manually with `./scripts/init_db.sh` or direct
`psql`, keep normal startup migration-free and apply any additive SQL files with
the deployment script/tool that owns that database. The explicit `migrate`
command refuses to run when IMS schemas already exist but SQLx migration `0001`
is not recorded, because `0001` rebuilds the IMS schemas.

`cuba-api` does not run migrations during normal startup by default. Use
`RUN_MIGRATIONS=true cargo run -p cuba-api` only when you intentionally want the
process startup to apply pending migrations, such as a one-off deployment step.
If an older local script or shell profile still exports `RUN_MIGRATIONS=true`,
set `RUN_MIGRATIONS=false` to keep startup read-only against an already
initialized database.

Migration policy and database-state checks:

- `migrations/0001_schema_final_ultimate_complete_v9.sql` is the frozen
  baseline; do not edit it after it has been applied.
- Add every later schema or seed-data change as a new migration.
- Check SQLx migration checksums with
  `./scripts/check_sqlx_migration_checksums.sh`.
- Verify the auth/system patch migrations with
  `./scripts/verify_patch_migrations.sh`.
- Repair guidance is in `docs/database-migrations.md`.

HTTP listen configuration:

- `IMS_BIND_ADDR` is the full listen address, for example `127.0.0.1:8081` or
  `0.0.0.0:8080`.
- `PORT` is used only when `IMS_BIND_ADDR` is not set; the default is
  `0.0.0.0:${PORT:-8080}`.

Shared database pool configuration for API and worker:

- `DB_MAX_CONN` defaults to `32`.
- `DB_MIN_CONN` defaults to `4` and must be less than or equal to `DB_MAX_CONN`.
- `DB_ACQUIRE_TIMEOUT_SECS` defaults to `5`.
- `DB_IDLE_TIMEOUT_SECS` defaults to `600`.
- `DB_MAX_LIFETIME_SECS` defaults to `1800`.

Worker scheduling configuration:

- `WORKER_MATERIALIZED_VIEW_REFRESH_MINUTES` defaults to `5`.
- `WORKER_LOW_STOCK_CHECK_MINUTES` defaults to `10`.
- `WORKER_MRP_RUN_MINUTES` defaults to `30`.
- `WORKER_AUDIT_CLEANUP_DAYS` defaults to `90`.

Authentication invalidation model:

- Access tokens are short-lived self-contained JWTs. The API validates
  signature, issuer, expiry, and token type, then trusts embedded roles and
  permissions until expiry.
- Default access-token TTL is 900 seconds (`JWT_EXPIRES_SECONDS`, 15 minutes).
  User disablement and permission changes take effect at login/refresh; already
  issued access tokens may remain valid until expiry.
- Refresh tokens are checked against PostgreSQL and rotated on every refresh, so
  replaying a previous refresh token is rejected.
- A strong immediate invalidation model would require per-request user lookup
  and an optional token-version claim; that is not the current runtime policy.

Frontend integration:

- Auth and permission guide: `docs/frontend-auth.md`

## Verify

```bash
cargo check --workspace
cargo test --workspace
```

## Phase 3 Master Data

- OpenAPI: `GET /api/openapi/master-data.json`
- Postman: `docs/postman/master-data.phase3.postman_collection.json`
- Bruno: `docs/bruno/master-data.phase3/`
- Smoke: `TOKEN=<write-or-admin-jwt> READ_TOKEN=<read-only-jwt> ./scripts/phase3_acceptance.sh`

---

## Module Deep Dive: cuba-reporting

### Module Structure

```text
crates/cuba-reporting/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── domain/
    │   ├── mod.rs
    │   ├── entities.rs
    │   ├── errors.rs
    │   └── value_objects.rs
    ├── application/
    │   ├── mod.rs
    │   ├── commands.rs
    │   ├── ports.rs
    │   └── services.rs
    ├── infrastructure/
    │   ├── mod.rs
    │   └── postgres.rs
    └── interface/
        ├── mod.rs
        ├── dto.rs
        ├── handlers.rs
        └── routes.rs
```

### API Endpoints

| Method | Endpoint | Description | Status |
|--------|----------|-------------|--------|
| GET | `/api/reporting/health` | Module health check | ✅ Active |
| GET | `/api/reporting/current-stock` | Current stock levels | 🚧 Placeholder |
| GET | `/api/reporting/inventory-value` | Inventory valuation | 🚧 Placeholder |
| GET | `/api/reporting/quality-status` | Quality inspection status | 🚧 Placeholder |
| GET | `/api/reporting/mrp-shortage` | MRP shortage analysis | 🚧 Placeholder |
| GET | `/api/reporting/low-stock-alert` | Low stock alerts | 🚧 Placeholder |
| GET | `/api/reporting/data-consistency` | Data consistency checks | 🚧 Placeholder |
| POST | `/api/reporting/refresh` | Refresh materialized views | ✅ Active |
